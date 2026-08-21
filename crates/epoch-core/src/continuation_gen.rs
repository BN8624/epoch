// M2.5 후속 세대 출생·다음 세대 권리 생성·검증 — RNG 없이 기존 Marriage B에서 파생

use crate::claim_propagation::{
    CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION, ClaimPropagationWorld, DerivedSuccessionClaim,
};
use crate::claim_propagation_gen::{
    generate_claim_propagation_world, validate_initial_claim_propagation,
};
use crate::context::{HouseIdentity, RealmIdentity};
use crate::continuation::{
    BIRTH_COUNT, BirthRecord, GENERATION_CONTINUATION_WORLD_SCHEMA_VERSION, GenerationContinuation,
    GenerationContinuationWorld, NEWBORN_COUNT, NEXT_GENERATION_CLAIM_COUNT, NewbornPerson,
    NextGenerationClaim,
};
use crate::error::CoreError;
use crate::family::Marriage;
use crate::political::ActiveRole;
use crate::population::{
    GenerationBand, HOUSE_COUNT, HOUSES_PER_REALM, House, PERSON_COUNT, Person,
};
use crate::rights::{ClaimBasis, ClaimStanding, RealmRights, SuccessionClaim};
use crate::world::WORLD_REALM_COUNT;
use std::collections::{BTreeMap, BTreeSet};

const NEXT_GENERATION_DISTANCE: u8 = 1;

fn invalid_continuation(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidContinuation(msg.into())
}

fn map_layer_error(err: CoreError) -> CoreError {
    match err {
        CoreError::InvalidWorld(msg) => invalid_continuation(format!("world: {msg}")),
        CoreError::InvalidPopulation(msg) => invalid_continuation(format!("population: {msg}")),
        CoreError::InvalidPolitical(msg) => invalid_continuation(format!("political: {msg}")),
        CoreError::InvalidContext(msg) => invalid_continuation(format!("context: {msg}")),
        CoreError::InvalidRights(msg) => invalid_continuation(format!("rights: {msg}")),
        CoreError::InvalidFamily(msg) => invalid_continuation(format!("family: {msg}")),
        CoreError::InvalidClaimPropagation(msg) => {
            invalid_continuation(format!("claim_propagation: {msg}"))
        }
        other => other,
    }
}

struct ContinuationLookups<'a> {
    person_by_id: BTreeMap<&'a str, &'a Person>,
    house_by_id: BTreeMap<&'a str, &'a House>,
    house_identity_by_id: BTreeMap<&'a str, &'a HouseIdentity>,
    realm_identity_by_id: BTreeMap<&'a str, &'a RealmIdentity>,
    realm_rights_by_id: BTreeMap<&'a str, &'a RealmRights>,
    claim_by_id: BTreeMap<&'a str, &'a SuccessionClaim>,
    derived_by_realm: BTreeMap<&'a str, &'a DerivedSuccessionClaim>,
    active_by_person: BTreeMap<&'a str, &'a crate::political::ActiveActor>,
    marriages: &'a [Marriage],
}

struct RealmContinuationRoles<'a> {
    realm_id: &'a str,
    h2: &'a House,
    h0_current: &'a Person,
    h2_head: &'a Person,
    marriage_a: &'a Marriage,
    marriage_b: &'a Marriage,
    restored: &'a SuccessionClaim,
    candidate_c: &'a DerivedSuccessionClaim,
    incumbent_person_id: &'a str,
}

fn continuation_lookups(base_world: &ClaimPropagationWorld) -> ContinuationLookups<'_> {
    let pop = &base_world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population;
    let mut derived_by_realm: BTreeMap<&str, &DerivedSuccessionClaim> = BTreeMap::new();
    for derived in &base_world.propagation.derived_claims {
        derived_by_realm.insert(derived.realm_id.as_str(), derived);
    }
    ContinuationLookups {
        person_by_id: pop.persons.iter().map(|p| (p.id.as_str(), p)).collect(),
        house_by_id: pop.houses.iter().map(|h| (h.id.as_str(), h)).collect(),
        house_identity_by_id: base_world
            .family_world
            .rights_world
            .context_world
            .context
            .house_identities
            .iter()
            .map(|h| (h.house_id.as_str(), h))
            .collect(),
        realm_identity_by_id: base_world
            .family_world
            .rights_world
            .context_world
            .context
            .realm_identities
            .iter()
            .map(|r| (r.realm_id.as_str(), r))
            .collect(),
        realm_rights_by_id: base_world
            .family_world
            .rights_world
            .rights
            .realms
            .iter()
            .map(|r| (r.realm_id.as_str(), r))
            .collect(),
        claim_by_id: base_world
            .family_world
            .rights_world
            .rights
            .claims
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect(),
        derived_by_realm,
        active_by_person: base_world
            .family_world
            .rights_world
            .context_world
            .political
            .roster
            .active_actors
            .iter()
            .map(|a| (a.person_id.as_str(), a))
            .collect(),
        marriages: &base_world.family_world.family.marriages,
    }
}

fn validate_base_world(base_world: &ClaimPropagationWorld) -> Result<(), CoreError> {
    if base_world.schema_version != CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION {
        return Err(invalid_continuation(format!(
            "base schema_version {} != {CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION}",
            base_world.schema_version
        )));
    }
    if base_world.seed != base_world.family_world.seed {
        return Err(invalid_continuation(format!(
            "base seed {} != family seed {}",
            base_world.seed, base_world.family_world.seed
        )));
    }
    validate_initial_claim_propagation(&base_world.family_world, &base_world.propagation)
        .map_err(map_layer_error)?;
    let pop = &base_world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population;
    if pop.persons.len() != PERSON_COUNT {
        return Err(invalid_continuation(format!(
            "population persons {} != {PERSON_COUNT}",
            pop.persons.len()
        )));
    }
    if pop.houses.len() != HOUSE_COUNT {
        return Err(invalid_continuation(format!(
            "houses {} != {HOUSE_COUNT}",
            pop.houses.len()
        )));
    }
    Ok(())
}

fn require_person<'a>(
    persons: &BTreeMap<&str, &'a Person>,
    id: &str,
    what: &str,
) -> Result<&'a Person, CoreError> {
    persons
        .get(id)
        .copied()
        .ok_or_else(|| invalid_continuation(format!("unknown {what} {id}")))
}

fn require_house<'a>(
    houses: &BTreeMap<&str, &'a House>,
    id: &str,
    what: &str,
) -> Result<&'a House, CoreError> {
    houses
        .get(id)
        .copied()
        .ok_or_else(|| invalid_continuation(format!("unknown {what} house {id}")))
}

fn require_unique<T>(
    items: Vec<&T>,
    missing: impl Into<String>,
    duplicate: impl Into<String>,
) -> Result<&T, CoreError> {
    let missing = missing.into();
    match items.len() {
        0 => Err(invalid_continuation(missing)),
        1 => items
            .into_iter()
            .next()
            .ok_or_else(|| invalid_continuation(missing)),
        _ => Err(invalid_continuation(duplicate.into())),
    }
}

fn sorted_pair(a: &str, b: &str, what: &str) -> Result<Vec<String>, CoreError> {
    if a == b {
        return Err(invalid_continuation(format!(
            "{what} pair is not distinct: {a}"
        )));
    }
    let mut pair = vec![a.to_string(), b.to_string()];
    pair.sort();
    Ok(pair)
}

fn require_sorted_unique_ids(ids: &[String], what: &str) -> Result<(), CoreError> {
    for window in ids.windows(2) {
        let left = window
            .first()
            .ok_or_else(|| invalid_continuation(format!("{what} window")))?;
        let right = window
            .get(1)
            .ok_or_else(|| invalid_continuation(format!("{what} window")))?;
        if left >= right {
            return Err(invalid_continuation(format!(
                "{what} not strictly sorted: {left} / {right}"
            )));
        }
    }
    Ok(())
}

fn sorted_realm_ids<'a>(lookups: &'a ContinuationLookups<'a>) -> Result<Vec<&'a str>, CoreError> {
    let mut realm_ids: Vec<&str> = lookups.realm_identity_by_id.keys().copied().collect();
    realm_ids.sort();
    if realm_ids.len() != WORLD_REALM_COUNT {
        return Err(invalid_continuation(format!(
            "realm identities {} != {WORLD_REALM_COUNT}",
            realm_ids.len()
        )));
    }
    Ok(realm_ids)
}

fn rank_id(prefix: &str, rank: usize) -> Result<String, CoreError> {
    let n = rank
        .checked_add(1)
        .ok_or_else(|| invalid_continuation(format!("{prefix} rank overflow")))?;
    Ok(format!("{prefix}-{n:02}"))
}

fn birth_id_for_rank(rank: usize) -> Result<String, CoreError> {
    rank_id("birth", rank)
}

fn next_claim_id_for_rank(rank: usize) -> Result<String, CoreError> {
    rank_id("next-claim", rank)
}

fn newborn_id_for_rank(rank: usize) -> Result<String, CoreError> {
    let n = PERSON_COUNT
        .checked_add(rank)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| invalid_continuation("newborn id overflow"))?;
    Ok(format!("person-{n:03}"))
}

fn find_marriage_by_spouses<'a>(
    marriages: &'a [Marriage],
    left: &str,
    right: &str,
    what: &str,
) -> Result<&'a Marriage, CoreError> {
    let expected = sorted_pair(left, right, what)?;
    let matches: Vec<&Marriage> = marriages
        .iter()
        .filter(|marriage| marriage.spouse_person_ids == expected)
        .collect();
    require_unique(
        matches,
        format!("{what} marriage missing"),
        format!("duplicate {what} marriages"),
    )
}

fn unique_original_claim<'a>(
    base_world: &'a ClaimPropagationWorld,
    realm_id: &str,
    basis: ClaimBasis,
    standing: ClaimStanding,
    label: &str,
) -> Result<&'a SuccessionClaim, CoreError> {
    let matches: Vec<&SuccessionClaim> = base_world
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .filter(|claim| {
            claim.realm_id == realm_id && claim.basis == basis && claim.standing == standing
        })
        .collect();
    require_unique(
        matches,
        format!("{label} original claim missing for {realm_id}"),
        format!("multiple {label} original claims for {realm_id}"),
    )
}

fn unique_actor_in_realm<'a>(
    lookups: &'a ContinuationLookups<'a>,
    realm_id: &str,
    role: ActiveRole,
    what: &str,
) -> Result<&'a crate::political::ActiveActor, CoreError> {
    let matches: Vec<&crate::political::ActiveActor> = lookups
        .active_by_person
        .values()
        .copied()
        .filter(|actor| actor.realm_id == realm_id && actor.primary_role == role)
        .collect();
    require_unique(
        matches,
        format!("{what} missing in {realm_id}"),
        format!("multiple {what} in {realm_id}"),
    )
}

fn classify_realm_roles<'a>(
    realm_id: &'a str,
    lookups: &'a ContinuationLookups<'a>,
    base_world: &'a ClaimPropagationWorld,
) -> Result<RealmContinuationRoles<'a>, CoreError> {
    let realm_rights = lookups
        .realm_rights_by_id
        .get(realm_id)
        .copied()
        .ok_or_else(|| invalid_continuation(format!("unknown Realm {realm_id}")))?;
    let incumbent = require_person(
        &lookups.person_by_id,
        realm_rights.incumbent_person_id.as_str(),
        "incumbent",
    )?;
    if incumbent.realm_id != realm_id {
        return Err(invalid_continuation(format!(
            "incumbent {} realm {} != {realm_id}",
            incumbent.id, incumbent.realm_id
        )));
    }
    let h0 = require_house(&lookups.house_by_id, incumbent.house_id.as_str(), "H0")?;
    if h0.realm_id != realm_id {
        return Err(invalid_continuation(format!(
            "H0 {} realm {} != {realm_id}",
            h0.id, h0.realm_id
        )));
    }
    if h0.head_person_id != incumbent.id {
        return Err(invalid_continuation(format!(
            "incumbent {} is not H0 {} head {}",
            incumbent.id, h0.id, h0.head_person_id
        )));
    }
    let ri = lookups
        .realm_identity_by_id
        .get(realm_id)
        .copied()
        .ok_or_else(|| invalid_continuation(format!("unknown realm identity {realm_id}")))?;

    let mut h1: Option<&House> = None;
    let mut h2: Option<&House> = None;
    let mut houses_in_realm = 0usize;
    for house in lookups.house_by_id.values().copied() {
        if house.realm_id != realm_id {
            continue;
        }
        houses_in_realm += 1;
        if house.id == h0.id {
            continue;
        }
        let hi = lookups
            .house_identity_by_id
            .get(house.id.as_str())
            .copied()
            .ok_or_else(|| invalid_continuation(format!("missing HouseIdentity {}", house.id)))?;
        let religious_minority =
            hi.culture_id == ri.majority_culture_id && hi.religion_id != ri.majority_religion_id;
        let cultural_minority =
            hi.culture_id != ri.majority_culture_id && hi.religion_id == ri.majority_religion_id;
        if religious_minority && cultural_minority {
            return Err(invalid_continuation(format!(
                "house {} matches both H1 and H2 identity",
                house.id
            )));
        }
        if religious_minority {
            if h1.is_some() {
                return Err(invalid_continuation(format!(
                    "realm {realm_id} has multiple religious-minority houses"
                )));
            }
            h1 = Some(house);
        } else if cultural_minority {
            if h2.is_some() {
                return Err(invalid_continuation(format!(
                    "realm {realm_id} has multiple cultural-minority houses"
                )));
            }
            h2 = Some(house);
        } else {
            return Err(invalid_continuation(format!(
                "house {} is neither H1 nor H2 of {realm_id}",
                house.id
            )));
        }
    }
    if houses_in_realm != HOUSES_PER_REALM {
        return Err(invalid_continuation(format!(
            "realm {realm_id} houses {houses_in_realm} != {HOUSES_PER_REALM}"
        )));
    }
    let h1 = h1.ok_or_else(|| {
        invalid_continuation(format!(
            "realm {realm_id} missing religious-minority house H1"
        ))
    })?;
    let h2 = h2.ok_or_else(|| {
        invalid_continuation(format!(
            "realm {realm_id} missing cultural-minority house H2"
        ))
    })?;
    if h0.id == h1.id || h0.id == h2.id || h1.id == h2.id {
        return Err(invalid_continuation(format!(
            "realm {realm_id} house roles are not distinct"
        )));
    }

    let h0_head = require_person(&lookups.person_by_id, h0.head_person_id.as_str(), "H0 head")?;
    let h1_head = require_person(&lookups.person_by_id, h1.head_person_id.as_str(), "H1 head")?;
    let h2_head = require_person(&lookups.person_by_id, h2.head_person_id.as_str(), "H2 head")?;
    if h2.head_person_id != h2_head.id {
        return Err(invalid_continuation(format!(
            "H2 head {} is not head of {}",
            h2_head.id, h2.id
        )));
    }

    let h0_current_actor = unique_actor_in_realm(
        lookups,
        realm_id,
        ActiveRole::RulingHouseCurrent,
        "H0 current",
    )?;
    let h0_current = require_person(
        &lookups.person_by_id,
        h0_current_actor.person_id.as_str(),
        "H0 current",
    )?;
    if h0_current.house_id != h0.id {
        return Err(invalid_continuation(format!(
            "H0 current {} house {} != H0 {}",
            h0_current.id, h0_current.house_id, h0.id
        )));
    }
    if h0_current.generation != GenerationBand::Current {
        return Err(invalid_continuation(format!(
            "H0 current {} is not Current",
            h0_current.id
        )));
    }
    if h2_head.generation != GenerationBand::Current {
        return Err(invalid_continuation(format!(
            "H2 head {} is not Current",
            h2_head.id
        )));
    }
    let h2_head_actor = lookups
        .active_by_person
        .get(h2_head.id.as_str())
        .copied()
        .ok_or_else(|| invalid_continuation(format!("H2 head {} is not Active", h2_head.id)))?;
    if h2_head_actor.primary_role != ActiveRole::HouseHead {
        return Err(invalid_continuation(format!(
            "H2 head {} primary role {:?} != HouseHead",
            h2_head.id, h2_head_actor.primary_role
        )));
    }

    let marriage_a = find_marriage_by_spouses(
        lookups.marriages,
        h0_head.id.as_str(),
        h1_head.id.as_str(),
        "Marriage A",
    )?;
    let marriage_b = find_marriage_by_spouses(
        lookups.marriages,
        h0_current.id.as_str(),
        h2_head.id.as_str(),
        "Marriage B",
    )?;
    if marriage_a.id == marriage_b.id {
        return Err(invalid_continuation(format!(
            "Marriage A and Marriage B are the same record {}",
            marriage_a.id
        )));
    }

    let restored = unique_original_claim(
        base_world,
        realm_id,
        ClaimBasis::RestoredLineRecord,
        ClaimStanding::Contested,
        "Restored",
    )?;
    if restored.claimant_person_id != h2_head.id {
        return Err(invalid_continuation(format!(
            "Restored claimant {} != H2 head {} in {realm_id}",
            restored.claimant_person_id, h2_head.id
        )));
    }
    if restored.claimant_house_id != h2.id {
        return Err(invalid_continuation(format!(
            "Restored claimant house {} != H2 {} in {realm_id}",
            restored.claimant_house_id, h2.id
        )));
    }
    let candidate_c = lookups
        .derived_by_realm
        .get(realm_id)
        .copied()
        .ok_or_else(|| {
            invalid_continuation(format!("Candidate C derived claim missing for {realm_id}"))
        })?;
    if candidate_c.source_claim_id != restored.id {
        return Err(invalid_continuation(format!(
            "Candidate C source {} != restored {} in {realm_id}",
            candidate_c.source_claim_id, restored.id
        )));
    }
    if candidate_c.via_parent_person_id != h2_head.id {
        return Err(invalid_continuation(format!(
            "Candidate C via parent {} != H2 head {} in {realm_id}",
            candidate_c.via_parent_person_id, h2_head.id
        )));
    }

    Ok(RealmContinuationRoles {
        realm_id,
        h2,
        h0_current,
        h2_head,
        marriage_a,
        marriage_b,
        restored,
        candidate_c,
        incumbent_person_id: realm_rights.incumbent_person_id.as_str(),
    })
}

fn classify_all_realms<'a>(
    base_world: &'a ClaimPropagationWorld,
    lookups: &'a ContinuationLookups<'a>,
) -> Result<Vec<RealmContinuationRoles<'a>>, CoreError> {
    let realm_ids = sorted_realm_ids(lookups)?;
    let mut roles = Vec::with_capacity(WORLD_REALM_COUNT);
    for realm_id in realm_ids {
        roles.push(classify_realm_roles(realm_id, lookups, base_world)?);
    }
    if roles.len() != WORLD_REALM_COUNT {
        return Err(invalid_continuation(format!(
            "classified realms {} != {WORLD_REALM_COUNT}",
            roles.len()
        )));
    }
    Ok(roles)
}

fn newborn_name(h2: &House, newborn_id: &str) -> Result<String, CoreError> {
    if h2.name.is_empty() {
        return Err(invalid_continuation(format!(
            "H2 house {} name is empty",
            h2.id
        )));
    }
    let name = format!("{} 후대 1", h2.name);
    if name.is_empty() {
        return Err(invalid_continuation(format!(
            "newborn {newborn_id} name is empty"
        )));
    }
    if name == newborn_id {
        return Err(invalid_continuation(format!(
            "newborn {newborn_id} name is the person id"
        )));
    }
    Ok(name)
}

/// ClaimPropagationWorld의 실제 Marriage B에서 후속 출생과 다음 세대 권리를 만든다. RNG를 쓰지 않는다.
pub fn derive_generation_continuation(
    base_world: &ClaimPropagationWorld,
) -> Result<GenerationContinuation, CoreError> {
    validate_base_world(base_world)?;
    let lookups = continuation_lookups(base_world);
    let roles = classify_all_realms(base_world, &lookups)?;

    let mut births = Vec::with_capacity(BIRTH_COUNT);
    let mut newborns = Vec::with_capacity(NEWBORN_COUNT);
    let mut derived_claims = Vec::with_capacity(NEXT_GENERATION_CLAIM_COUNT);

    for (rank, role) in roles.iter().enumerate() {
        let birth_id = birth_id_for_rank(rank)?;
        let child_id = newborn_id_for_rank(rank)?;
        let claim_id = next_claim_id_for_rank(rank)?;
        let h2_identity = lookups
            .house_identity_by_id
            .get(role.h2.id.as_str())
            .copied()
            .ok_or_else(|| invalid_continuation(format!("missing HouseIdentity {}", role.h2.id)))?;
        let parent_person_ids = role.marriage_b.spouse_person_ids.clone();
        require_sorted_unique_ids(&parent_person_ids, "parent_person_ids")?;
        if parent_person_ids.len() != 2 {
            return Err(invalid_continuation(format!(
                "Marriage B {} parent count {} != 2",
                role.marriage_b.id,
                parent_person_ids.len()
            )));
        }
        if parent_person_ids
            .iter()
            .any(|id| id == role.incumbent_person_id)
        {
            return Err(invalid_continuation(format!(
                "incumbent {} is a birth parent in {}",
                role.incumbent_person_id, role.realm_id
            )));
        }
        if lookups.person_by_id.contains_key(child_id.as_str()) {
            return Err(invalid_continuation(format!(
                "newborn id {child_id} collides with existing Person"
            )));
        }
        if role.candidate_c.claimant_person_id == child_id {
            return Err(invalid_continuation(format!(
                "Candidate C {} collides with newborn {child_id}",
                role.candidate_c.claimant_person_id
            )));
        }

        births.push(BirthRecord {
            id: birth_id,
            realm_id: role.realm_id.to_string(),
            marriage_id: role.marriage_b.id.clone(),
            child_person_id: child_id.clone(),
            parent_person_ids,
        });
        newborns.push(NewbornPerson {
            id: child_id.clone(),
            name: newborn_name(role.h2, &child_id)?,
            realm_id: role.h2.realm_id.clone(),
            house_id: role.h2.id.clone(),
            home_territory_id: role.h2.seat_territory_id.clone(),
            culture_id: h2_identity.culture_id.clone(),
            religion_id: h2_identity.religion_id.clone(),
        });
        derived_claims.push(NextGenerationClaim {
            id: claim_id,
            realm_id: role.realm_id.to_string(),
            succession_target_key: role.restored.succession_target_key.clone(),
            claimant_person_id: child_id,
            claimant_house_id: role.h2.id.clone(),
            source_claim_id: role.restored.id.clone(),
            via_parent_person_id: role.h2_head.id.clone(),
            generation_distance: NEXT_GENERATION_DISTANCE,
        });
    }

    births.sort_by(|a, b| a.id.cmp(&b.id));
    newborns.sort_by(|a, b| a.id.cmp(&b.id));
    derived_claims.sort_by(|a, b| a.id.cmp(&b.id));
    let continuation = GenerationContinuation {
        births,
        newborns,
        derived_claims,
    };
    validate_generation_continuation(base_world, &continuation)?;
    Ok(continuation)
}

/// seed에서 GenerationContinuationWorld를 생성한다.
pub fn generate_generation_continuation_world(
    seed: u64,
) -> Result<GenerationContinuationWorld, CoreError> {
    let base_world = generate_claim_propagation_world(seed).map_err(map_layer_error)?;
    if base_world.seed != seed {
        return Err(invalid_continuation(format!(
            "base seed {} != requested {seed}",
            base_world.seed
        )));
    }
    let continuation = derive_generation_continuation(&base_world)?;
    let world = GenerationContinuationWorld {
        schema_version: GENERATION_CONTINUATION_WORLD_SCHEMA_VERSION,
        seed,
        base_world,
        continuation,
    };
    if world.schema_version != GENERATION_CONTINUATION_WORLD_SCHEMA_VERSION {
        return Err(invalid_continuation(format!(
            "wrong schema {}",
            world.schema_version
        )));
    }
    if world.seed != world.base_world.seed {
        return Err(invalid_continuation(format!(
            "seed mismatch {} / {}",
            world.seed, world.base_world.seed
        )));
    }
    Ok(world)
}

/// 후속 출생·신생아·다음 세대 권리 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_generation_continuation(
    base_world: &ClaimPropagationWorld,
    continuation: &GenerationContinuation,
) -> Result<(), CoreError> {
    validate_base_world(base_world)?;
    let lookups = continuation_lookups(base_world);
    let roles = classify_all_realms(base_world, &lookups)?;
    let realm_ids = sorted_realm_ids(&lookups)?;

    if continuation.births.len() != BIRTH_COUNT {
        return Err(invalid_continuation(format!(
            "births {} != {BIRTH_COUNT}",
            continuation.births.len()
        )));
    }
    if continuation.newborns.len() != NEWBORN_COUNT {
        return Err(invalid_continuation(format!(
            "newborns {} != {NEWBORN_COUNT}",
            continuation.newborns.len()
        )));
    }
    if continuation.derived_claims.len() != NEXT_GENERATION_CLAIM_COUNT {
        return Err(invalid_continuation(format!(
            "new claims {} != {NEXT_GENERATION_CLAIM_COUNT}",
            continuation.derived_claims.len()
        )));
    }

    let birth_ids: Vec<String> = continuation.births.iter().map(|b| b.id.clone()).collect();
    let newborn_ids: Vec<String> = continuation.newborns.iter().map(|n| n.id.clone()).collect();
    let claim_ids: Vec<String> = continuation
        .derived_claims
        .iter()
        .map(|c| c.id.clone())
        .collect();
    require_sorted_unique_ids(&birth_ids, "births")?;
    require_sorted_unique_ids(&newborn_ids, "newborns")?;
    require_sorted_unique_ids(&claim_ids, "derived claims")?;

    let mut birth_by_id: BTreeMap<&str, &BirthRecord> = BTreeMap::new();
    let mut newborn_by_id: BTreeMap<&str, &NewbornPerson> = BTreeMap::new();
    let mut claim_by_newborn: BTreeMap<&str, &NextGenerationClaim> = BTreeMap::new();
    let mut seen_birth_ids: BTreeSet<&str> = BTreeSet::new();
    let mut seen_newborn_ids: BTreeSet<&str> = BTreeSet::new();
    let mut seen_claim_ids: BTreeSet<&str> = BTreeSet::new();
    let mut seen_names: BTreeSet<&str> = BTreeSet::new();
    let mut births_by_realm: BTreeMap<&str, usize> = BTreeMap::new();
    let mut marriage_b_used = 0usize;
    let mut restored_sources = 0usize;

    for (idx, birth) in continuation.births.iter().enumerate() {
        let expected_id = birth_id_for_rank(idx)?;
        if birth.id != expected_id {
            return Err(invalid_continuation(format!(
                "birth id {} != {expected_id}",
                birth.id
            )));
        }
        if !seen_birth_ids.insert(birth.id.as_str()) {
            return Err(invalid_continuation(format!(
                "duplicate birth {}",
                birth.id
            )));
        }
        let expected_realm = realm_ids.get(idx).copied().ok_or_else(|| {
            invalid_continuation(format!(
                "birth {} index {idx} has no corresponding realm",
                birth.id
            ))
        })?;
        if birth.realm_id != expected_realm {
            return Err(invalid_continuation(format!(
                "birth {} realm {} != {expected_realm}",
                birth.id, birth.realm_id
            )));
        }
        let role = roles.get(idx).ok_or_else(|| {
            invalid_continuation(format!("birth {} has no classified realm roles", birth.id))
        })?;
        if role.realm_id != birth.realm_id {
            return Err(invalid_continuation(format!(
                "birth {} classified realm {} != {}",
                birth.id, role.realm_id, birth.realm_id
            )));
        }
        lookups
            .realm_identity_by_id
            .get(birth.realm_id.as_str())
            .ok_or_else(|| invalid_continuation(format!("unknown Realm {}", birth.realm_id)))?;

        let marriage = lookups
            .marriages
            .iter()
            .find(|m| m.id == birth.marriage_id)
            .ok_or_else(|| {
                invalid_continuation(format!(
                    "unknown marriage {} for {}",
                    birth.marriage_id, birth.id
                ))
            })?;
        if birth.marriage_id == role.marriage_a.id {
            return Err(invalid_continuation(format!(
                "Marriage A {} used for birth {}",
                role.marriage_a.id, birth.id
            )));
        }
        if birth.marriage_id != role.marriage_b.id {
            return Err(invalid_continuation(format!(
                "Marriage B semantic mismatch: birth {} marriage {} != {}",
                birth.id, birth.marriage_id, role.marriage_b.id
            )));
        }
        marriage_b_used += 1;
        if birth.parent_person_ids.len() != 2 {
            return Err(invalid_continuation(format!(
                "parent count {} != 2 for {}",
                birth.parent_person_ids.len(),
                birth.id
            )));
        }
        require_sorted_unique_ids(&birth.parent_person_ids, &format!("{} parents", birth.id))?;
        if birth.parent_person_ids != marriage.spouse_person_ids {
            return Err(invalid_continuation(format!(
                "spouse pair mismatch for {} vs {}",
                birth.id, marriage.id
            )));
        }
        if birth.parent_person_ids != role.marriage_b.spouse_person_ids {
            return Err(invalid_continuation(format!(
                "spouse pair mismatch for {} vs Marriage B {}",
                birth.id, role.marriage_b.id
            )));
        }

        let mut parent_houses: BTreeSet<&str> = BTreeSet::new();
        let mut parent_realms: BTreeSet<&str> = BTreeSet::new();
        for parent_id in &birth.parent_person_ids {
            let parent = require_person(&lookups.person_by_id, parent_id.as_str(), "parent")?;
            if parent.generation != GenerationBand::Current {
                return Err(invalid_continuation(format!(
                    "parent {parent_id} is not Current"
                )));
            }
            if parent.id == role.incumbent_person_id {
                return Err(invalid_continuation(format!(
                    "parent {parent_id} == incumbent"
                )));
            }
            parent_houses.insert(parent.house_id.as_str());
            parent_realms.insert(parent.realm_id.as_str());
        }
        if parent_houses.len() != 2 {
            return Err(invalid_continuation(format!(
                "parents of {} are not from different houses",
                birth.id
            )));
        }
        if parent_realms.len() != 1 || !parent_realms.contains(birth.realm_id.as_str()) {
            return Err(invalid_continuation(format!(
                "parents of {} are not the same Realm",
                birth.id
            )));
        }
        if !birth
            .parent_person_ids
            .iter()
            .any(|id| id == &role.h2_head.id)
        {
            return Err(invalid_continuation(format!(
                "source parent {} not in birth pair {}",
                role.h2_head.id, birth.id
            )));
        }
        if !birth
            .parent_person_ids
            .iter()
            .any(|id| id == &role.h0_current.id)
        {
            return Err(invalid_continuation(format!(
                "H0 current {} not in birth pair {}",
                role.h0_current.id, birth.id
            )));
        }
        if birth_by_id.insert(birth.id.as_str(), birth).is_some() {
            return Err(invalid_continuation(format!(
                "duplicate birth {}",
                birth.id
            )));
        }
        *births_by_realm.entry(birth.realm_id.as_str()).or_insert(0) += 1;
    }

    if marriage_b_used != BIRTH_COUNT {
        return Err(invalid_continuation(format!(
            "Marriage B used {marriage_b_used} != {BIRTH_COUNT}"
        )));
    }
    if births_by_realm.len() != WORLD_REALM_COUNT {
        return Err(invalid_continuation(format!(
            "birth realm coverage {} != {WORLD_REALM_COUNT}",
            births_by_realm.len()
        )));
    }
    for (realm_id, count) in &births_by_realm {
        if *count != 1 {
            return Err(invalid_continuation(format!(
                "realm {realm_id} births {count} != 1"
            )));
        }
    }

    for (idx, newborn) in continuation.newborns.iter().enumerate() {
        let expected_id = newborn_id_for_rank(idx)?;
        if newborn.id != expected_id {
            return Err(invalid_continuation(format!(
                "newborn id {} != {expected_id}",
                newborn.id
            )));
        }
        if !seen_newborn_ids.insert(newborn.id.as_str()) {
            return Err(invalid_continuation(format!(
                "duplicate newborn {}",
                newborn.id
            )));
        }
        if lookups.person_by_id.contains_key(newborn.id.as_str()) {
            return Err(invalid_continuation(format!(
                "existing Person ID collision {}",
                newborn.id
            )));
        }
        let expected_realm = realm_ids.get(idx).copied().ok_or_else(|| {
            invalid_continuation(format!(
                "newborn {} index {idx} has no corresponding realm",
                newborn.id
            ))
        })?;
        if newborn.realm_id != expected_realm {
            return Err(invalid_continuation(format!(
                "wrong Realm {} for newborn {}",
                newborn.realm_id, newborn.id
            )));
        }
        let role = roles.get(idx).ok_or_else(|| {
            invalid_continuation(format!(
                "newborn {} has no classified realm roles",
                newborn.id
            ))
        })?;
        if newborn.house_id != role.h2.id {
            return Err(invalid_continuation(format!(
                "wrong H2 House {} for newborn {} expected {}",
                newborn.house_id, newborn.id, role.h2.id
            )));
        }
        if newborn.realm_id != role.h2.realm_id {
            return Err(invalid_continuation(format!(
                "newborn {} realm {} != H2 realm {}",
                newborn.id, newborn.realm_id, role.h2.realm_id
            )));
        }
        if newborn.home_territory_id != role.h2.seat_territory_id {
            return Err(invalid_continuation(format!(
                "newborn {} seat {} != H2 seat {}",
                newborn.id, newborn.home_territory_id, role.h2.seat_territory_id
            )));
        }
        let h2_identity = lookups
            .house_identity_by_id
            .get(role.h2.id.as_str())
            .copied()
            .ok_or_else(|| invalid_continuation(format!("missing HouseIdentity {}", role.h2.id)))?;
        if newborn.culture_id != h2_identity.culture_id {
            return Err(invalid_continuation(format!(
                "newborn {} culture {} != H2 culture {}",
                newborn.id, newborn.culture_id, h2_identity.culture_id
            )));
        }
        if newborn.religion_id != h2_identity.religion_id {
            return Err(invalid_continuation(format!(
                "newborn {} religion {} != H2 religion {}",
                newborn.id, newborn.religion_id, h2_identity.religion_id
            )));
        }
        if newborn.name.is_empty() {
            return Err(invalid_continuation(format!(
                "newborn {} name is empty",
                newborn.id
            )));
        }
        if newborn.name == newborn.id {
            return Err(invalid_continuation(format!(
                "newborn {} name is the person id",
                newborn.id
            )));
        }
        let expected_name = newborn_name(role.h2, newborn.id.as_str())?;
        if newborn.name != expected_name {
            return Err(invalid_continuation(format!(
                "newborn {} name {} != {expected_name}",
                newborn.id, newborn.name
            )));
        }
        if !seen_names.insert(newborn.name.as_str()) {
            return Err(invalid_continuation(format!(
                "duplicate newborn name {}",
                newborn.name
            )));
        }
        let birth = continuation
            .births
            .iter()
            .find(|b| b.child_person_id == newborn.id)
            .ok_or_else(|| {
                invalid_continuation(format!("birth missing for newborn {}", newborn.id))
            })?;
        if birth.realm_id != newborn.realm_id {
            return Err(invalid_continuation(format!(
                "newborn {} realm {} != birth realm {}",
                newborn.id, newborn.realm_id, birth.realm_id
            )));
        }
        if role.candidate_c.claimant_person_id == newborn.id {
            return Err(invalid_continuation(format!(
                "Candidate C/newborn accidental identity collision {}",
                newborn.id
            )));
        }
        if newborn_by_id.insert(newborn.id.as_str(), newborn).is_some() {
            return Err(invalid_continuation(format!(
                "duplicate newborn ID {}",
                newborn.id
            )));
        }
    }

    for (idx, claim) in continuation.derived_claims.iter().enumerate() {
        let expected_id = next_claim_id_for_rank(idx)?;
        if claim.id != expected_id {
            return Err(invalid_continuation(format!(
                "claim id {} != {expected_id}",
                claim.id
            )));
        }
        if !seen_claim_ids.insert(claim.id.as_str()) {
            return Err(invalid_continuation(format!(
                "duplicate claim {}",
                claim.id
            )));
        }
        let expected_realm = realm_ids.get(idx).copied().ok_or_else(|| {
            invalid_continuation(format!(
                "claim {} index {idx} has no corresponding realm",
                claim.id
            ))
        })?;
        if claim.realm_id != expected_realm {
            return Err(invalid_continuation(format!(
                "wrong Realm {} for claim {}",
                claim.realm_id, claim.id
            )));
        }
        let role = roles.get(idx).ok_or_else(|| {
            invalid_continuation(format!("claim {} has no classified realm roles", claim.id))
        })?;
        let source = lookups
            .claim_by_id
            .get(claim.source_claim_id.as_str())
            .copied()
            .ok_or_else(|| {
                invalid_continuation(format!(
                    "source claim missing {} for {}",
                    claim.source_claim_id, claim.id
                ))
            })?;
        match source.basis {
            ClaimBasis::RestoredLineRecord => restored_sources += 1,
            ClaimBasis::DirectDescent => {
                return Err(invalid_continuation(format!(
                    "source claim {} for {} is not Restored/Contested",
                    source.id, claim.id
                )));
            }
        }
        if source.standing != ClaimStanding::Contested {
            return Err(invalid_continuation(format!(
                "source claim {} for {} is not Restored/Contested",
                source.id, claim.id
            )));
        }
        if source.id != role.restored.id {
            return Err(invalid_continuation(format!(
                "source claim {} != restored {} for {}",
                source.id, role.restored.id, claim.id
            )));
        }
        if claim.via_parent_person_id != source.claimant_person_id {
            return Err(invalid_continuation(format!(
                "via parent {} != source claimant {} for {}",
                claim.via_parent_person_id, source.claimant_person_id, claim.id
            )));
        }
        if claim.via_parent_person_id != role.h2_head.id {
            return Err(invalid_continuation(format!(
                "via parent {} != H2 head {} for {}",
                claim.via_parent_person_id, role.h2_head.id, claim.id
            )));
        }
        let birth = continuation
            .births
            .iter()
            .find(|b| b.realm_id == claim.realm_id)
            .ok_or_else(|| invalid_continuation(format!("birth missing for claim {}", claim.id)))?;
        if !birth
            .parent_person_ids
            .iter()
            .any(|id| id == &claim.via_parent_person_id)
        {
            return Err(invalid_continuation(format!(
                "source parent {} not in birth pair {}",
                claim.via_parent_person_id, birth.id
            )));
        }
        let newborn = newborn_by_id
            .get(claim.claimant_person_id.as_str())
            .copied()
            .ok_or_else(|| {
                invalid_continuation(format!(
                    "claimant {} is not a newborn for {}",
                    claim.claimant_person_id, claim.id
                ))
            })?;
        if claim.claimant_person_id != birth.child_person_id
            || claim.claimant_person_id != newborn.id
        {
            return Err(invalid_continuation(format!(
                "claimant {} != newborn {} for {}",
                claim.claimant_person_id, newborn.id, claim.id
            )));
        }
        if claim.claimant_house_id != newborn.house_id || newborn.house_id != role.h2.id {
            return Err(invalid_continuation(format!(
                "claimant house {} != H2 {} for {}",
                claim.claimant_house_id, role.h2.id, claim.id
            )));
        }
        if claim.generation_distance != NEXT_GENERATION_DISTANCE {
            return Err(invalid_continuation(format!(
                "generation distance {} != {NEXT_GENERATION_DISTANCE} for {}",
                claim.generation_distance, claim.id
            )));
        }
        if claim.succession_target_key != source.succession_target_key {
            return Err(invalid_continuation(format!(
                "target mismatch {} != {} for {}",
                claim.succession_target_key, source.succession_target_key, claim.id
            )));
        }
        if claim.realm_id != source.realm_id {
            return Err(invalid_continuation(format!(
                "claim {} realm {} != source realm {}",
                claim.id, claim.realm_id, source.realm_id
            )));
        }
        if role.candidate_c.claimant_person_id == newborn.id {
            return Err(invalid_continuation(format!(
                "Candidate C/newborn accidental identity collision {}",
                newborn.id
            )));
        }
        if role.candidate_c.claimant_house_id != newborn.house_id {
            return Err(invalid_continuation(format!(
                "Candidate C house {} != newborn house {}",
                role.candidate_c.claimant_house_id, newborn.house_id
            )));
        }
        if role.candidate_c.source_claim_id != claim.source_claim_id {
            return Err(invalid_continuation(format!(
                "Candidate C source {} != newborn source {}",
                role.candidate_c.source_claim_id, claim.source_claim_id
            )));
        }
        if role.candidate_c.via_parent_person_id != claim.via_parent_person_id {
            return Err(invalid_continuation(format!(
                "Candidate C via parent {} != newborn via parent {}",
                role.candidate_c.via_parent_person_id, claim.via_parent_person_id
            )));
        }
        if claim_by_newborn
            .insert(claim.claimant_person_id.as_str(), claim)
            .is_some()
        {
            return Err(invalid_continuation(format!(
                "duplicate claim claimant {}",
                claim.claimant_person_id
            )));
        }
    }

    if restored_sources != NEXT_GENERATION_CLAIM_COUNT {
        return Err(invalid_continuation(format!(
            "restored sources {restored_sources} != {NEXT_GENERATION_CLAIM_COUNT}"
        )));
    }
    Ok(())
}
