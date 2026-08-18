// 통치자 사망을 입력으로 법적 우선 후보와 공석을 계산한다

use crate::claim_propagation::{
    CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION, ClaimPropagationWorld, DERIVED_GENERATION_DISTANCE,
    DerivedSuccessionClaim,
};
use crate::claim_propagation_gen::{
    generate_claim_propagation_world, validate_initial_claim_propagation,
};
use crate::error::CoreError;
use crate::familygen::effective_parent_ids;
use crate::political::ActiveRole;
use crate::population::{GenerationBand, House, Person};
use crate::rights::{
    ClaimBasis, ClaimStanding, RealmRights, SuccessionClaim, succession_target_key,
};
use crate::succession::{
    IncumbentDeath, RealmVacancy, SUCCESSION_CANDIDATE_COUNT, SUCCESSION_WORLD_SCHEMA_VERSION,
    SuccessionCandidate, SuccessionClaimOrigin, SuccessionPriority, SuccessionTransition,
    SuccessionWorld,
};
use std::collections::{BTreeMap, BTreeSet};

fn invalid_succession(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidSuccession(msg.into())
}

fn map_layer_error(err: CoreError) -> CoreError {
    match err {
        CoreError::InvalidWorld(msg) => invalid_succession(format!("world: {msg}")),
        CoreError::InvalidPopulation(msg) => invalid_succession(format!("population: {msg}")),
        CoreError::InvalidPolitical(msg) => invalid_succession(format!("political: {msg}")),
        CoreError::InvalidContext(msg) => invalid_succession(format!("context: {msg}")),
        CoreError::InvalidRights(msg) => invalid_succession(format!("rights: {msg}")),
        CoreError::InvalidFamily(msg) => invalid_succession(format!("family: {msg}")),
        CoreError::InvalidClaimPropagation(msg) => {
            invalid_succession(format!("claim_propagation: {msg}"))
        }
        other => other,
    }
}

struct SuccessionLookups<'a> {
    person_by_id: BTreeMap<&'a str, &'a Person>,
    house_by_id: BTreeMap<&'a str, &'a House>,
    realm_ids: BTreeSet<&'a str>,
    claim_by_id: BTreeMap<&'a str, &'a SuccessionClaim>,
    derived_by_id: BTreeMap<&'a str, &'a DerivedSuccessionClaim>,
    realm_rights_by_id: BTreeMap<&'a str, &'a RealmRights>,
    active_by_person: BTreeMap<&'a str, &'a crate::political::ActiveActor>,
    supporting: BTreeSet<&'a str>,
}

fn succession_lookups(world: &ClaimPropagationWorld) -> SuccessionLookups<'_> {
    let pop = &world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population;
    let skeleton = &world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .world;
    SuccessionLookups {
        person_by_id: pop.persons.iter().map(|p| (p.id.as_str(), p)).collect(),
        house_by_id: pop.houses.iter().map(|h| (h.id.as_str(), h)).collect(),
        realm_ids: skeleton.realms.iter().map(|r| r.id.as_str()).collect(),
        claim_by_id: world
            .family_world
            .rights_world
            .rights
            .claims
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect(),
        derived_by_id: world
            .propagation
            .derived_claims
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect(),
        realm_rights_by_id: world
            .family_world
            .rights_world
            .rights
            .realms
            .iter()
            .map(|r| (r.realm_id.as_str(), r))
            .collect(),
        active_by_person: world
            .family_world
            .rights_world
            .context_world
            .political
            .roster
            .active_actors
            .iter()
            .map(|a| (a.person_id.as_str(), a))
            .collect(),
        supporting: world
            .family_world
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

fn validate_pre_world(world: &ClaimPropagationWorld) -> Result<(), CoreError> {
    if world.schema_version != CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION {
        return Err(invalid_succession(format!(
            "pre-succession schema_version {} != {CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION}",
            world.schema_version
        )));
    }
    if world.seed != world.family_world.seed {
        return Err(invalid_succession(format!(
            "pre-succession seed {} != family seed {}",
            world.seed, world.family_world.seed
        )));
    }
    validate_initial_claim_propagation(&world.family_world, &world.propagation)
        .map_err(map_layer_error)?;
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
        .ok_or_else(|| invalid_succession(format!("{what} person {id} missing")))
}

fn require_house<'a>(
    houses: &BTreeMap<&str, &'a House>,
    id: &str,
    what: &str,
) -> Result<&'a House, CoreError> {
    houses
        .get(id)
        .copied()
        .ok_or_else(|| invalid_succession(format!("{what} house {id} missing")))
}

fn death_id_for_realm(realm_id: &str) -> String {
    format!("death:{realm_id}:incumbent")
}

fn find_unique_original_claim<'a>(
    world: &'a ClaimPropagationWorld,
    realm_id: &str,
    basis: ClaimBasis,
    standing: ClaimStanding,
    label: &str,
) -> Result<&'a SuccessionClaim, CoreError> {
    let matches: Vec<&SuccessionClaim> = world
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .filter(|claim| {
            claim.realm_id == realm_id && claim.basis == basis && claim.standing == standing
        })
        .collect();
    match matches.len() {
        0 => Err(invalid_succession(format!(
            "{label} original claim missing for {realm_id}"
        ))),
        1 => matches.into_iter().next().ok_or_else(|| {
            invalid_succession(format!("{label} original claim missing for {realm_id}"))
        }),
        _ => Err(invalid_succession(format!(
            "automatic candidate replacement forbidden: multiple {label} original claims for {realm_id}"
        ))),
    }
}

fn find_unique_derived_claim<'a>(
    world: &'a ClaimPropagationWorld,
    realm_id: &str,
    source_claim_id: &str,
) -> Result<&'a DerivedSuccessionClaim, CoreError> {
    let matches: Vec<&DerivedSuccessionClaim> = world
        .propagation
        .derived_claims
        .iter()
        .filter(|claim| claim.realm_id == realm_id)
        .collect();
    match matches.len() {
        0 => Err(invalid_succession(format!(
            "derived claim missing for {realm_id}"
        ))),
        1 => {
            let derived = matches.into_iter().next().ok_or_else(|| {
                invalid_succession(format!("derived claim missing for {realm_id}"))
            })?;
            if derived.source_claim_id != source_claim_id {
                return Err(invalid_succession(format!(
                    "derived {} source {} is not restored original {source_claim_id}",
                    derived.id, derived.source_claim_id
                )));
            }
            Ok(derived)
        }
        _ => Err(invalid_succession(format!(
            "automatic candidate replacement forbidden: multiple derived claims for {realm_id}"
        ))),
    }
}

fn candidate_from_original(
    claim: &SuccessionClaim,
    origin: SuccessionClaimOrigin,
    priority: SuccessionPriority,
) -> Result<SuccessionCandidate, CoreError> {
    if origin != SuccessionClaimOrigin::Original {
        return Err(invalid_succession(format!(
            "original claim {} origin is not original",
            claim.id
        )));
    }
    Ok(SuccessionCandidate {
        person_id: claim.claimant_person_id.clone(),
        house_id: claim.claimant_house_id.clone(),
        claim_record_id: claim.id.clone(),
        claim_origin: origin,
        priority,
        generation_distance: 0,
    })
}

fn candidate_from_derived(
    derived: &DerivedSuccessionClaim,
) -> Result<SuccessionCandidate, CoreError> {
    if derived.generation_distance != DERIVED_GENERATION_DISTANCE {
        return Err(invalid_succession(format!(
            "derived {} generation_distance {} != {DERIVED_GENERATION_DISTANCE}",
            derived.id, derived.generation_distance
        )));
    }
    Ok(SuccessionCandidate {
        person_id: derived.claimant_person_id.clone(),
        house_id: derived.claimant_house_id.clone(),
        claim_record_id: derived.id.clone(),
        claim_origin: SuccessionClaimOrigin::Derived,
        priority: SuccessionPriority::RestoredContestedDerived,
        generation_distance: derived.generation_distance,
    })
}

fn sort_candidates(candidates: &mut [SuccessionCandidate]) {
    candidates.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.person_id.cmp(&b.person_id))
    });
}

fn require_sorted_candidates(candidates: &[SuccessionCandidate]) -> Result<(), CoreError> {
    for window in candidates.windows(2) {
        let a = window
            .first()
            .ok_or_else(|| invalid_succession("candidate sort window"))?;
        let b = window
            .get(1)
            .ok_or_else(|| invalid_succession("candidate sort window"))?;
        let ka = (a.priority, a.person_id.as_str());
        let kb = (b.priority, b.person_id.as_str());
        if ka >= kb {
            return Err(invalid_succession(format!(
                "candidate order unstable: {} / {}",
                a.person_id, b.person_id
            )));
        }
    }
    Ok(())
}

/// 해당 Realm의 실제 incumbent 사망을 입력으로 3인 후보와 공석을 계산한다.
///
/// 중첩된 기존 세계는 변경하지 않는다. 사망 후 상태는 transition이 원본이다.
pub fn resolve_incumbent_death(
    world: &ClaimPropagationWorld,
    realm_id: &str,
) -> Result<SuccessionTransition, CoreError> {
    validate_pre_world(world)?;
    let lookups = succession_lookups(world);
    if !lookups.realm_ids.contains(realm_id) {
        return Err(invalid_succession(format!("unknown realm {realm_id}")));
    }
    let realm_rights = lookups
        .realm_rights_by_id
        .get(realm_id)
        .copied()
        .ok_or_else(|| invalid_succession(format!("incumbent rights missing for {realm_id}")))?;
    let incumbent = require_person(
        &lookups.person_by_id,
        realm_rights.incumbent_person_id.as_str(),
        "incumbent",
    )?;
    let expected_target = succession_target_key(realm_id);
    if realm_rights.succession_target_key != expected_target {
        return Err(invalid_succession(format!(
            "succession target mismatch: rights {} != {expected_target}",
            realm_rights.succession_target_key
        )));
    }

    let direct = find_unique_original_claim(
        world,
        realm_id,
        ClaimBasis::DirectDescent,
        ClaimStanding::Strong,
        "Direct",
    )?;
    let restored = find_unique_original_claim(
        world,
        realm_id,
        ClaimBasis::RestoredLineRecord,
        ClaimStanding::Contested,
        "Restored",
    )?;
    let derived = find_unique_derived_claim(world, realm_id, restored.id.as_str())?;

    let mut candidates = vec![
        candidate_from_original(
            direct,
            SuccessionClaimOrigin::Original,
            SuccessionPriority::DirectStrongOriginal,
        )?,
        candidate_from_original(
            restored,
            SuccessionClaimOrigin::Original,
            SuccessionPriority::RestoredContestedOriginal,
        )?,
        candidate_from_derived(derived)?,
    ];
    sort_candidates(&mut candidates);

    let winner = candidates
        .iter()
        .find(|c| c.priority == SuccessionPriority::DirectStrongOriginal)
        .ok_or_else(|| {
            invalid_succession(format!(
                "DirectStrongOriginal successor missing for {realm_id}"
            ))
        })?
        .clone();

    let transition = SuccessionTransition {
        realm_id: realm_id.to_string(),
        succession_target_key: expected_target,
        death: IncumbentDeath {
            id: death_id_for_realm(realm_id),
            realm_id: realm_id.to_string(),
            person_id: incumbent.id.clone(),
        },
        candidates,
        presumptive_successor_person_id: winner.person_id,
        presumptive_successor_house_id: winner.house_id,
        vacancy: RealmVacancy {
            realm_id: realm_id.to_string(),
            former_incumbent_person_id: incumbent.id.clone(),
            is_vacant: true,
        },
    };
    validate_succession_transition(world, &transition)?;
    Ok(transition)
}

/// seed와 Realm에서 SuccessionWorld를 생성한다. 이 계층은 RNG를 쓰지 않는다.
pub fn generate_succession_world(seed: u64, realm_id: &str) -> Result<SuccessionWorld, CoreError> {
    let pre_succession_world = generate_claim_propagation_world(seed).map_err(map_layer_error)?;
    if pre_succession_world.seed != seed {
        return Err(invalid_succession(format!(
            "pre-succession seed {} != requested {seed}",
            pre_succession_world.seed
        )));
    }
    let transition = resolve_incumbent_death(&pre_succession_world, realm_id)?;
    Ok(SuccessionWorld {
        schema_version: SUCCESSION_WORLD_SCHEMA_VERSION,
        seed,
        pre_succession_world,
        transition,
    })
}

/// 계승 전환 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_succession_transition(
    world: &ClaimPropagationWorld,
    transition: &SuccessionTransition,
) -> Result<(), CoreError> {
    validate_pre_world(world)?;
    let lookups = succession_lookups(world);

    if !lookups.realm_ids.contains(transition.realm_id.as_str()) {
        return Err(invalid_succession(format!(
            "unknown realm {}",
            transition.realm_id
        )));
    }
    let realm_rights = lookups
        .realm_rights_by_id
        .get(transition.realm_id.as_str())
        .copied()
        .ok_or_else(|| {
            invalid_succession(format!(
                "incumbent rights missing for {}",
                transition.realm_id
            ))
        })?;
    let expected_target = succession_target_key(transition.realm_id.as_str());
    if transition.succession_target_key != expected_target
        || realm_rights.succession_target_key != expected_target
    {
        return Err(invalid_succession(format!(
            "succession target mismatch: transition {} rights {} expected {expected_target}",
            transition.succession_target_key, realm_rights.succession_target_key
        )));
    }
    if transition.death.realm_id != transition.realm_id {
        return Err(invalid_succession(format!(
            "death realm {} != transition realm {}",
            transition.death.realm_id, transition.realm_id
        )));
    }
    if transition.death.id != death_id_for_realm(transition.realm_id.as_str()) {
        return Err(invalid_succession(format!(
            "death id {} != {}",
            transition.death.id,
            death_id_for_realm(transition.realm_id.as_str())
        )));
    }
    if transition.death.person_id != realm_rights.incumbent_person_id {
        return Err(invalid_succession(format!(
            "death person {} != incumbent {}",
            transition.death.person_id, realm_rights.incumbent_person_id
        )));
    }
    let deceased = require_person(
        &lookups.person_by_id,
        transition.death.person_id.as_str(),
        "deceased",
    )?;
    if deceased.realm_id != transition.realm_id {
        return Err(invalid_succession(format!(
            "deceased {} realm {} != {}",
            deceased.id, deceased.realm_id, transition.realm_id
        )));
    }
    if transition.vacancy.realm_id != transition.realm_id {
        return Err(invalid_succession(format!(
            "vacancy realm {} != {}",
            transition.vacancy.realm_id, transition.realm_id
        )));
    }
    if !transition.vacancy.is_vacant {
        return Err(invalid_succession(format!(
            "vacancy false for {}",
            transition.realm_id
        )));
    }
    if transition.vacancy.former_incumbent_person_id != deceased.id {
        return Err(invalid_succession(format!(
            "former incumbent {} != deceased {}",
            transition.vacancy.former_incumbent_person_id, deceased.id
        )));
    }
    if transition.candidates.len() != SUCCESSION_CANDIDATE_COUNT {
        return Err(invalid_succession(format!(
            "candidates {} != {SUCCESSION_CANDIDATE_COUNT}",
            transition.candidates.len()
        )));
    }
    require_sorted_candidates(&transition.candidates)?;

    let expected_direct = find_unique_original_claim(
        world,
        transition.realm_id.as_str(),
        ClaimBasis::DirectDescent,
        ClaimStanding::Strong,
        "Direct",
    )?;
    let expected_restored = find_unique_original_claim(
        world,
        transition.realm_id.as_str(),
        ClaimBasis::RestoredLineRecord,
        ClaimStanding::Contested,
        "Restored",
    )?;
    let expected_derived = find_unique_derived_claim(
        world,
        transition.realm_id.as_str(),
        expected_restored.id.as_str(),
    )?;

    let mut seen_persons: BTreeSet<&str> = BTreeSet::new();
    let mut seen_claims: BTreeSet<&str> = BTreeSet::new();
    let mut seen_priorities: BTreeSet<SuccessionPriority> = BTreeSet::new();
    let mut seen_origins_by_priority: BTreeSet<(SuccessionPriority, SuccessionClaimOrigin)> =
        BTreeSet::new();
    let mut winner: Option<&SuccessionCandidate> = None;

    for candidate in &transition.candidates {
        if !seen_persons.insert(candidate.person_id.as_str()) {
            return Err(invalid_succession(format!(
                "duplicate candidate person {}",
                candidate.person_id
            )));
        }
        if !seen_claims.insert(candidate.claim_record_id.as_str()) {
            return Err(invalid_succession(format!(
                "duplicate candidate claim {}",
                candidate.claim_record_id
            )));
        }
        if !seen_priorities.insert(candidate.priority) {
            return Err(invalid_succession(format!(
                "automatic candidate replacement forbidden: duplicate priority {:?} for {}",
                candidate.priority, transition.realm_id
            )));
        }
        if !seen_origins_by_priority.insert((candidate.priority, candidate.claim_origin)) {
            return Err(invalid_succession(format!(
                "duplicate candidate origin/priority for {}",
                candidate.person_id
            )));
        }
        if candidate.person_id == deceased.id {
            return Err(invalid_succession(format!(
                "deceased incumbent {} is a candidate",
                deceased.id
            )));
        }
        let person = require_person(
            &lookups.person_by_id,
            candidate.person_id.as_str(),
            "candidate",
        )?;
        if person.realm_id != transition.realm_id {
            return Err(invalid_succession(format!(
                "candidate {} wrong realm {} != {}",
                person.id, person.realm_id, transition.realm_id
            )));
        }
        if candidate.house_id != person.house_id {
            return Err(invalid_succession(format!(
                "candidate {} house {} != person house {}",
                person.id, candidate.house_id, person.house_id
            )));
        }
        let _house = require_house(
            &lookups.house_by_id,
            candidate.house_id.as_str(),
            "candidate",
        )?;

        match (candidate.claim_origin, candidate.priority) {
            (SuccessionClaimOrigin::Original, SuccessionPriority::DirectStrongOriginal) => {
                if candidate.generation_distance != 0 {
                    return Err(invalid_succession(format!(
                        "generation distance {} for Direct original {}",
                        candidate.generation_distance, candidate.person_id
                    )));
                }
                if candidate.claim_record_id != expected_direct.id {
                    return Err(invalid_succession(format!(
                        "automatic candidate replacement forbidden: Direct claim {} != {}",
                        candidate.claim_record_id, expected_direct.id
                    )));
                }
                if expected_direct.claimant_person_id != candidate.person_id
                    || expected_direct.claimant_house_id != candidate.house_id
                {
                    return Err(invalid_succession(format!(
                        "Direct original claimant mismatch for {}",
                        candidate.person_id
                    )));
                }
                if expected_direct.succession_target_key != transition.succession_target_key {
                    return Err(invalid_succession(format!(
                        "candidate {} wrong target {}",
                        candidate.person_id, expected_direct.succession_target_key
                    )));
                }
                if expected_direct.realm_id != transition.realm_id {
                    return Err(invalid_succession(format!(
                        "candidate {} wrong realm {}",
                        candidate.person_id, expected_direct.realm_id
                    )));
                }
                if person.generation != GenerationBand::Young {
                    return Err(invalid_succession(format!(
                        "Direct original {} is not Young",
                        person.id
                    )));
                }
                if !lookups.supporting.contains(person.id.as_str()) {
                    return Err(invalid_succession(format!(
                        "Direct original {} is not Supporting",
                        person.id
                    )));
                }
                if !person.known_parent_ids.iter().any(|id| id == &deceased.id) {
                    return Err(invalid_succession(format!(
                        "Direct original {} is not a known child of incumbent {}",
                        person.id, deceased.id
                    )));
                }
            }
            (SuccessionClaimOrigin::Original, SuccessionPriority::RestoredContestedOriginal) => {
                if candidate.generation_distance != 0 {
                    return Err(invalid_succession(format!(
                        "generation distance {} for Restored original {}",
                        candidate.generation_distance, candidate.person_id
                    )));
                }
                if candidate.claim_record_id != expected_restored.id {
                    return Err(invalid_succession(format!(
                        "automatic candidate replacement forbidden: Restored claim {} != {}",
                        candidate.claim_record_id, expected_restored.id
                    )));
                }
                if expected_restored.claimant_person_id != candidate.person_id
                    || expected_restored.claimant_house_id != candidate.house_id
                {
                    return Err(invalid_succession(format!(
                        "Restored original claimant mismatch for {}",
                        candidate.person_id
                    )));
                }
                if expected_restored.succession_target_key != transition.succession_target_key {
                    return Err(invalid_succession(format!(
                        "candidate {} wrong target {}",
                        candidate.person_id, expected_restored.succession_target_key
                    )));
                }
                if expected_restored.realm_id != transition.realm_id {
                    return Err(invalid_succession(format!(
                        "candidate {} wrong realm {}",
                        candidate.person_id, expected_restored.realm_id
                    )));
                }
                let actor = lookups
                    .active_by_person
                    .get(person.id.as_str())
                    .copied()
                    .ok_or_else(|| {
                        invalid_succession(format!(
                            "Restored original {} is not Active HouseHead",
                            person.id
                        ))
                    })?;
                if actor.primary_role != ActiveRole::HouseHead {
                    return Err(invalid_succession(format!(
                        "Restored original {} primary role {:?} != HouseHead",
                        person.id, actor.primary_role
                    )));
                }
                let house =
                    require_house(&lookups.house_by_id, person.house_id.as_str(), "restored")?;
                if house.head_person_id != person.id {
                    return Err(invalid_succession(format!(
                        "Restored original {} is not head of {}",
                        person.id, house.id
                    )));
                }
            }
            (SuccessionClaimOrigin::Derived, SuccessionPriority::RestoredContestedDerived) => {
                if candidate.generation_distance != DERIVED_GENERATION_DISTANCE {
                    return Err(invalid_succession(format!(
                        "generation distance {} for derived {}",
                        candidate.generation_distance, candidate.person_id
                    )));
                }
                if candidate.claim_record_id != expected_derived.id {
                    return Err(invalid_succession(format!(
                        "automatic candidate replacement forbidden: derived claim {} != {}",
                        candidate.claim_record_id, expected_derived.id
                    )));
                }
                let derived = lookups
                    .derived_by_id
                    .get(candidate.claim_record_id.as_str())
                    .copied()
                    .ok_or_else(|| {
                        invalid_succession(format!(
                            "derived claim {} missing",
                            candidate.claim_record_id
                        ))
                    })?;
                if derived.source_claim_id != expected_restored.id {
                    return Err(invalid_succession(format!(
                        "derived source {} is not Restored original {}",
                        derived.source_claim_id, expected_restored.id
                    )));
                }
                let source = lookups
                    .claim_by_id
                    .get(derived.source_claim_id.as_str())
                    .copied()
                    .ok_or_else(|| {
                        invalid_succession(format!(
                            "derived source claim {} missing",
                            derived.source_claim_id
                        ))
                    })?;
                if source.basis != ClaimBasis::RestoredLineRecord
                    || source.standing != ClaimStanding::Contested
                {
                    return Err(invalid_succession(format!(
                        "derived source {} is not Restored original",
                        source.id
                    )));
                }
                if derived.claimant_person_id != candidate.person_id
                    || derived.claimant_house_id != candidate.house_id
                {
                    return Err(invalid_succession(format!(
                        "derived claimant mismatch for {}",
                        candidate.person_id
                    )));
                }
                if derived.realm_id != transition.realm_id {
                    return Err(invalid_succession(format!(
                        "candidate {} wrong realm {}",
                        candidate.person_id, derived.realm_id
                    )));
                }
                if derived.succession_target_key != transition.succession_target_key {
                    return Err(invalid_succession(format!(
                        "candidate {} wrong target {}",
                        candidate.person_id, derived.succession_target_key
                    )));
                }
                if person.generation != GenerationBand::Young {
                    return Err(invalid_succession(format!(
                        "derived candidate {} is not Young",
                        person.id
                    )));
                }
                if !lookups.supporting.contains(person.id.as_str()) {
                    return Err(invalid_succession(format!(
                        "derived candidate {} is not Supporting",
                        person.id
                    )));
                }
                let parents = effective_parent_ids(
                    &world.family_world.rights_world,
                    &world.family_world.family,
                    person.id.as_str(),
                )
                .map_err(map_layer_error)?;
                if !parents
                    .iter()
                    .any(|id| id == &expected_restored.claimant_person_id)
                {
                    return Err(invalid_succession(format!(
                        "derived {} effective parents do not include Restored original {}",
                        person.id, expected_restored.claimant_person_id
                    )));
                }
            }
            (origin, priority) => {
                return Err(invalid_succession(format!(
                    "candidate {} origin/priority mismatch {origin:?}/{priority:?}",
                    candidate.person_id
                )));
            }
        }

        if candidate.priority == SuccessionPriority::DirectStrongOriginal {
            winner = Some(candidate);
        }
    }

    if !seen_priorities.contains(&SuccessionPriority::DirectStrongOriginal) {
        return Err(invalid_succession("Direct original missing"));
    }
    if !seen_priorities.contains(&SuccessionPriority::RestoredContestedOriginal) {
        return Err(invalid_succession("Restored original missing"));
    }
    if !seen_priorities.contains(&SuccessionPriority::RestoredContestedDerived) {
        return Err(invalid_succession("derived missing"));
    }

    let winner =
        winner.ok_or_else(|| invalid_succession("successor is not DirectStrongOriginal"))?;
    if transition.presumptive_successor_person_id != winner.person_id {
        return Err(invalid_succession(format!(
            "successor {} is not a DirectStrongOriginal candidate",
            transition.presumptive_successor_person_id
        )));
    }
    if transition.presumptive_successor_house_id != winner.house_id {
        return Err(invalid_succession(format!(
            "successor house {} != candidate house {}",
            transition.presumptive_successor_house_id, winner.house_id
        )));
    }
    if !seen_persons.contains(transition.presumptive_successor_person_id.as_str()) {
        return Err(invalid_succession(format!(
            "successor {} is not a candidate",
            transition.presumptive_successor_person_id
        )));
    }
    if transition.presumptive_successor_person_id == deceased.id {
        return Err(invalid_succession(format!(
            "successor {} is the deceased incumbent",
            deceased.id
        )));
    }
    Ok(())
}
