// M2.2 1세대 권리 전파 생성·검증 — RNG 없이 기존 Family parentage에서 파생

use crate::claim_propagation::{
    CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION, ClaimPropagationWorld, DERIVED_CLAIM_COUNT,
    DERIVED_GENERATION_DISTANCE, DerivedSuccessionClaim, InitialClaimPropagation,
};
use crate::error::CoreError;
use crate::family::{FAMILY_WORLD_SCHEMA_VERSION, FamilyWorld};
use crate::familygen::{effective_parent_ids, generate_family_world, validate_initial_family};
use crate::political::ActiveRole;
use crate::population::{GenerationBand, House, Person};
use crate::rights::{ClaimBasis, ClaimStanding, SUCCESSION_CLAIM_COUNT, SuccessionClaim};
use crate::world::WORLD_REALM_COUNT;
use std::collections::{BTreeMap, BTreeSet};

fn invalid_propagation(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidClaimPropagation(msg.into())
}

fn map_layer_error(err: CoreError) -> CoreError {
    match err {
        CoreError::InvalidWorld(msg) => invalid_propagation(format!("world: {msg}")),
        CoreError::InvalidPopulation(msg) => invalid_propagation(format!("population: {msg}")),
        CoreError::InvalidPolitical(msg) => invalid_propagation(format!("political: {msg}")),
        CoreError::InvalidContext(msg) => invalid_propagation(format!("context: {msg}")),
        CoreError::InvalidRights(msg) => invalid_propagation(format!("rights: {msg}")),
        CoreError::InvalidFamily(msg) => invalid_propagation(format!("family: {msg}")),
        other => other,
    }
}

fn validate_family_world(family_world: &FamilyWorld) -> Result<(), CoreError> {
    if family_world.schema_version != FAMILY_WORLD_SCHEMA_VERSION {
        return Err(invalid_propagation(format!(
            "family schema_version {} != {FAMILY_WORLD_SCHEMA_VERSION}",
            family_world.schema_version
        )));
    }
    if family_world.seed != family_world.rights_world.seed {
        return Err(invalid_propagation(format!(
            "family seed {} != rights seed {}",
            family_world.seed, family_world.rights_world.seed
        )));
    }
    validate_initial_family(&family_world.rights_world, &family_world.family)
        .map_err(map_layer_error)?;
    Ok(())
}

struct PropagationLookups<'a> {
    person_by_id: BTreeMap<&'a str, &'a Person>,
    house_by_id: BTreeMap<&'a str, &'a House>,
    house_identity_by_id: BTreeMap<&'a str, &'a crate::context::HouseIdentity>,
    realm_identity_by_id: BTreeMap<&'a str, &'a crate::context::RealmIdentity>,
    claim_by_id: BTreeMap<&'a str, &'a SuccessionClaim>,
    active_by_person: BTreeMap<&'a str, &'a crate::political::ActiveActor>,
    supporting: BTreeSet<&'a str>,
}

fn propagation_lookups(family_world: &FamilyWorld) -> PropagationLookups<'_> {
    let pop = &family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population;
    PropagationLookups {
        person_by_id: pop.persons.iter().map(|p| (p.id.as_str(), p)).collect(),
        house_by_id: pop.houses.iter().map(|h| (h.id.as_str(), h)).collect(),
        house_identity_by_id: family_world
            .rights_world
            .context_world
            .context
            .house_identities
            .iter()
            .map(|h| (h.house_id.as_str(), h))
            .collect(),
        realm_identity_by_id: family_world
            .rights_world
            .context_world
            .context
            .realm_identities
            .iter()
            .map(|r| (r.realm_id.as_str(), r))
            .collect(),
        claim_by_id: family_world
            .rights_world
            .rights
            .claims
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect(),
        active_by_person: family_world
            .rights_world
            .context_world
            .political
            .roster
            .active_actors
            .iter()
            .map(|a| (a.person_id.as_str(), a))
            .collect(),
        supporting: family_world
            .rights_world
            .context_world
            .political
            .roster
            .supporting_person_ids
            .iter()
            .map(|s| s.as_str())
            .collect(),
    }
}

fn require_person<'a>(
    persons: &BTreeMap<&str, &'a Person>,
    id: &str,
    what: &str,
) -> Result<&'a Person, CoreError> {
    persons
        .get(id)
        .copied()
        .ok_or_else(|| invalid_propagation(format!("unknown {what} {id}")))
}

fn require_house<'a>(
    houses: &BTreeMap<&str, &'a House>,
    id: &str,
    what: &str,
) -> Result<&'a House, CoreError> {
    houses
        .get(id)
        .copied()
        .ok_or_else(|| invalid_propagation(format!("unknown {what} house {id}")))
}

fn require_sorted_unique_ids(ids: &[String], what: &str) -> Result<(), CoreError> {
    for window in ids.windows(2) {
        let a = window
            .first()
            .ok_or_else(|| invalid_propagation(format!("{what} window")))?;
        let b = window
            .get(1)
            .ok_or_else(|| invalid_propagation(format!("{what} window")))?;
        if a >= b {
            return Err(invalid_propagation(format!(
                "{what} not strictly sorted: {a} / {b}"
            )));
        }
    }
    Ok(())
}

fn is_cultural_minority_house(
    house_id: &str,
    realm_id: &str,
    lookups: &PropagationLookups<'_>,
) -> Result<bool, CoreError> {
    let hi = lookups
        .house_identity_by_id
        .get(house_id)
        .copied()
        .ok_or_else(|| invalid_propagation(format!("unknown house identity {house_id}")))?;
    let ri = lookups
        .realm_identity_by_id
        .get(realm_id)
        .copied()
        .ok_or_else(|| invalid_propagation(format!("unknown realm identity {realm_id}")))?;
    Ok(hi.culture_id != ri.majority_culture_id && hi.religion_id == ri.majority_religion_id)
}

fn expected_h2_young_id(house: &House) -> Result<&str, CoreError> {
    house.member_ids.get(5).map(String::as_str).ok_or_else(|| {
        invalid_propagation(format!(
            "H2 house {} missing member_ids[5] expected child",
            house.id
        ))
    })
}

/// FamilyWorld의 원본 claim과 parentage에서 1세대 파생 권리를 만든다. RNG를 쓰지 않는다.
pub fn derive_initial_claim_propagation(
    family_world: &FamilyWorld,
) -> Result<InitialClaimPropagation, CoreError> {
    validate_family_world(family_world)?;
    let lookups = propagation_lookups(family_world);

    let mut seen_candidates: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut seen_provenance: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut candidates: Vec<DerivedSuccessionClaim> = Vec::new();

    for claim in &family_world.rights_world.rights.claims {
        let parent = require_person(
            &lookups.person_by_id,
            claim.claimant_person_id.as_str(),
            "source claimant",
        )?;
        for link in &family_world.family.parentages {
            if !link
                .parent_person_ids
                .iter()
                .any(|id| id == &claim.claimant_person_id)
            {
                continue;
            }
            let child = require_person(
                &lookups.person_by_id,
                link.child_person_id.as_str(),
                "child",
            )?;
            let candidate_key = (claim.realm_id.clone(), claim.id.clone(), child.id.clone());
            if !seen_candidates.insert(candidate_key) {
                continue;
            }
            let provenance = (
                claim.id.clone(),
                claim.claimant_person_id.clone(),
                child.id.clone(),
            );
            if !seen_provenance.insert(provenance) {
                continue;
            }
            candidates.push(DerivedSuccessionClaim {
                id: String::new(),
                realm_id: claim.realm_id.clone(),
                succession_target_key: claim.succession_target_key.clone(),
                claimant_person_id: child.id.clone(),
                claimant_house_id: child.house_id.clone(),
                source_claim_id: claim.id.clone(),
                via_parent_person_id: parent.id.clone(),
                generation_distance: DERIVED_GENERATION_DISTANCE,
            });
        }
    }

    candidates.sort_by(|a, b| {
        a.realm_id
            .cmp(&b.realm_id)
            .then(a.source_claim_id.cmp(&b.source_claim_id))
            .then(a.claimant_person_id.cmp(&b.claimant_person_id))
    });
    for (idx, derived) in candidates.iter_mut().enumerate() {
        derived.id = format!("derived-claim-{:02}", idx + 1);
    }

    let propagation = InitialClaimPropagation {
        derived_claims: candidates,
    };
    validate_initial_claim_propagation(family_world, &propagation)?;
    Ok(propagation)
}

/// seed에서 ClaimPropagationWorld를 생성한다.
pub fn generate_claim_propagation_world(seed: u64) -> Result<ClaimPropagationWorld, CoreError> {
    let family_world = generate_family_world(seed)?;
    if family_world.seed != seed {
        return Err(invalid_propagation(format!(
            "family seed {} != requested {seed}",
            family_world.seed
        )));
    }
    let propagation = derive_initial_claim_propagation(&family_world)?;
    Ok(ClaimPropagationWorld {
        schema_version: CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION,
        seed,
        family_world,
        propagation,
    })
}

/// 1세대 파생 권리 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_initial_claim_propagation(
    family_world: &FamilyWorld,
    propagation: &InitialClaimPropagation,
) -> Result<(), CoreError> {
    validate_family_world(family_world)?;
    let lookups = propagation_lookups(family_world);

    if family_world.rights_world.rights.claims.len() != SUCCESSION_CLAIM_COUNT {
        return Err(invalid_propagation(format!(
            "original claims {} != {SUCCESSION_CLAIM_COUNT}",
            family_world.rights_world.rights.claims.len()
        )));
    }
    if propagation.derived_claims.len() != DERIVED_CLAIM_COUNT {
        return Err(invalid_propagation(format!(
            "derived claims {} != {DERIVED_CLAIM_COUNT}",
            propagation.derived_claims.len()
        )));
    }

    let derived_ids: Vec<String> = propagation
        .derived_claims
        .iter()
        .map(|c| c.id.clone())
        .collect();
    require_sorted_unique_ids(&derived_ids, "derived_claims")?;

    let mut seen_ids: BTreeSet<&str> = BTreeSet::new();
    let mut seen_provenance: BTreeSet<(&str, &str, &str)> = BTreeSet::new();
    let mut derived_by_realm: BTreeMap<&str, usize> = BTreeMap::new();

    for (idx, derived) in propagation.derived_claims.iter().enumerate() {
        let expected_id = format!("derived-claim-{:02}", idx + 1);
        if derived.id != expected_id {
            return Err(invalid_propagation(format!(
                "derived id {} != {expected_id}",
                derived.id
            )));
        }
        if !seen_ids.insert(derived.id.as_str()) {
            return Err(invalid_propagation(format!(
                "duplicate derived id {}",
                derived.id
            )));
        }
        if derived.generation_distance != DERIVED_GENERATION_DISTANCE {
            return Err(invalid_propagation(format!(
                "derived {} generation_distance {} != {DERIVED_GENERATION_DISTANCE}",
                derived.id, derived.generation_distance
            )));
        }

        let source = lookups
            .claim_by_id
            .get(derived.source_claim_id.as_str())
            .copied()
            .ok_or_else(|| {
                invalid_propagation(format!(
                    "unknown source claim {} for {}",
                    derived.source_claim_id, derived.id
                ))
            })?;
        if source.basis != ClaimBasis::RestoredLineRecord {
            return Err(invalid_propagation(format!(
                "source claim {} for {} is not RestoredLineRecord",
                source.id, derived.id
            )));
        }
        if source.standing != ClaimStanding::Contested {
            return Err(invalid_propagation(format!(
                "source claim {} for {} is not Contested",
                source.id, derived.id
            )));
        }
        if source.claimant_person_id != derived.via_parent_person_id {
            return Err(invalid_propagation(format!(
                "source claimant {} != via parent {} for {}",
                source.claimant_person_id, derived.via_parent_person_id, derived.id
            )));
        }
        if derived.realm_id != source.realm_id {
            return Err(invalid_propagation(format!(
                "derived {} realm {} != source realm {}",
                derived.id, derived.realm_id, source.realm_id
            )));
        }
        if derived.succession_target_key != source.succession_target_key {
            return Err(invalid_propagation(format!(
                "derived {} target {} != source target {}",
                derived.id, derived.succession_target_key, source.succession_target_key
            )));
        }

        let via_parent = require_person(
            &lookups.person_by_id,
            derived.via_parent_person_id.as_str(),
            "parent",
        )?;
        let child = require_person(
            &lookups.person_by_id,
            derived.claimant_person_id.as_str(),
            "claimant",
        )?;
        if via_parent.realm_id != source.realm_id {
            return Err(invalid_propagation(format!(
                "via parent {} realm {} != source realm {}",
                via_parent.id, via_parent.realm_id, source.realm_id
            )));
        }
        if child.realm_id != source.realm_id {
            return Err(invalid_propagation(format!(
                "child {} realm {} != source realm {}",
                child.id, child.realm_id, source.realm_id
            )));
        }
        if derived.realm_id != child.realm_id {
            return Err(invalid_propagation(format!(
                "derived {} realm {} != child realm {}",
                derived.id, derived.realm_id, child.realm_id
            )));
        }
        if derived.claimant_house_id != child.house_id {
            return Err(invalid_propagation(format!(
                "derived {} claimant_house_id {} != child house {}",
                derived.id, derived.claimant_house_id, child.house_id
            )));
        }
        if child.generation != GenerationBand::Young {
            return Err(invalid_propagation(format!(
                "derived claimant {} is not Young",
                child.id
            )));
        }
        if !lookups.supporting.contains(child.id.as_str()) {
            return Err(invalid_propagation(format!(
                "derived claimant {} is not Supporting",
                child.id
            )));
        }
        let source_actor = lookups
            .active_by_person
            .get(via_parent.id.as_str())
            .copied()
            .ok_or_else(|| {
                invalid_propagation(format!(
                    "source parent {} is not Active HouseHead",
                    via_parent.id
                ))
            })?;
        if source_actor.primary_role != ActiveRole::HouseHead {
            return Err(invalid_propagation(format!(
                "source parent {} primary role {:?} != HouseHead",
                via_parent.id, source_actor.primary_role
            )));
        }

        let parent_house = require_house(
            &lookups.house_by_id,
            via_parent.house_id.as_str(),
            "source parent",
        )?;
        if parent_house.head_person_id != via_parent.id {
            return Err(invalid_propagation(format!(
                "source parent {} is not head of house {}",
                via_parent.id, parent_house.id
            )));
        }
        if !is_cultural_minority_house(
            parent_house.id.as_str(),
            source.realm_id.as_str(),
            &lookups,
        )? {
            return Err(invalid_propagation(format!(
                "source parent house {} is not H2 of {}",
                parent_house.id, source.realm_id
            )));
        }
        if child.house_id != parent_house.id {
            return Err(invalid_propagation(format!(
                "child {} house {} != source parent H2 {}",
                child.id, child.house_id, parent_house.id
            )));
        }
        let expected_child = expected_h2_young_id(parent_house)?;
        if child.id != expected_child {
            return Err(invalid_propagation(format!(
                "derived claimant {} is not H2 expected child {expected_child}",
                child.id
            )));
        }

        let parents = effective_parent_ids(
            &family_world.rights_world,
            &family_world.family,
            child.id.as_str(),
        )
        .map_err(map_layer_error)?;
        if !parents.iter().any(|id| id == &via_parent.id) {
            return Err(invalid_propagation(format!(
                "effective parents of {} do not include source parent {}",
                child.id, via_parent.id
            )));
        }

        let provenance = (
            derived.source_claim_id.as_str(),
            derived.via_parent_person_id.as_str(),
            derived.claimant_person_id.as_str(),
        );
        if !seen_provenance.insert(provenance) {
            return Err(invalid_propagation(format!(
                "duplicate provenance {:?} for {}",
                provenance, derived.id
            )));
        }
        *derived_by_realm
            .entry(derived.realm_id.as_str())
            .or_insert(0) += 1;
    }

    if derived_by_realm.len() != WORLD_REALM_COUNT {
        return Err(invalid_propagation(format!(
            "derived realm coverage {} != {WORLD_REALM_COUNT}",
            derived_by_realm.len()
        )));
    }
    for (realm_id, count) in &derived_by_realm {
        if *count != 1 {
            return Err(invalid_propagation(format!(
                "realm {realm_id} derived claims {count} != 1"
            )));
        }
    }
    Ok(())
}
