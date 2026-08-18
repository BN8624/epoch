// M2.3 통치자 사망·법적 우선 후계·공석과 3인 계승 주장 통합 테스트

use epoch_core::{
    ActiveRole, CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION, CONTEXT_WORLD_SCHEMA_VERSION, ClaimBasis,
    ClaimStanding, CoreError, DERIVED_GENERATION_DISTANCE, DYNASTIC_WORLD_SCHEMA_VERSION,
    FAMILY_WORLD_SCHEMA_VERSION, GenerationBand, POLITICAL_WORLD_SCHEMA_VERSION,
    RIGHTS_WORLD_SCHEMA_VERSION, SAVE_SCHEMA_VERSION, SUCCESSION_CANDIDATE_COUNT,
    SUCCESSION_WORLD_SCHEMA_VERSION, SuccessionClaimOrigin, SuccessionPriority,
    SuccessionTransition, SuccessionWorld, WORLD_SCHEMA_VERSION, effective_parent_ids,
    generate_claim_propagation_world, generate_dynastic_world, generate_family_world,
    generate_rights_world, generate_succession_world, resolve_incumbent_death,
    validate_succession_transition,
};
use std::collections::{BTreeMap, BTreeSet};

mod common;

const SEEDS: [u64; 5] = [0, 1, 2, 42, u64::MAX];
const SUCCESSION_CHECK_1: &str = "SUCCESSION_OK seed=1 realms=6 candidates=18 direct_winners=6 restored_winners=0 derived_winners=0 vacancies=6 realm01_bytes=71915";
const SUCCESSION_CHECK_2: &str = "SUCCESSION_OK seed=2 realms=6 candidates=18 direct_winners=6 restored_winners=0 derived_winners=0 vacancies=6 realm01_bytes=71914";

fn person_by_id(
    world: &epoch_core::ClaimPropagationWorld,
) -> BTreeMap<&str, &epoch_core::population::Person> {
    world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect()
}

fn house_by_id(
    world: &epoch_core::ClaimPropagationWorld,
) -> BTreeMap<&str, &epoch_core::population::House> {
    world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population
        .houses
        .iter()
        .map(|h| (h.id.as_str(), h))
        .collect()
}

fn house_identity_by_id(
    world: &epoch_core::ClaimPropagationWorld,
) -> BTreeMap<&str, &epoch_core::HouseIdentity> {
    world
        .family_world
        .rights_world
        .context_world
        .context
        .house_identities
        .iter()
        .map(|h| (h.house_id.as_str(), h))
        .collect()
}

fn realm_identity_by_id(
    world: &epoch_core::ClaimPropagationWorld,
) -> BTreeMap<&str, &epoch_core::RealmIdentity> {
    world
        .family_world
        .rights_world
        .context_world
        .context
        .realm_identities
        .iter()
        .map(|r| (r.realm_id.as_str(), r))
        .collect()
}

fn active_by_person(
    world: &epoch_core::ClaimPropagationWorld,
) -> BTreeMap<&str, &epoch_core::ActiveActor> {
    world
        .family_world
        .rights_world
        .context_world
        .political
        .roster
        .active_actors
        .iter()
        .map(|a| (a.person_id.as_str(), a))
        .collect()
}

fn supporting_ids(world: &epoch_core::ClaimPropagationWorld) -> BTreeSet<&str> {
    world
        .family_world
        .rights_world
        .context_world
        .political
        .roster
        .supporting_person_ids
        .iter()
        .map(|s| s.as_str())
        .collect()
}

fn classify_realm_houses<'a>(
    world: &'a epoch_core::ClaimPropagationWorld,
    realm_id: &str,
) -> (
    &'a epoch_core::population::House,
    &'a epoch_core::population::House,
    &'a epoch_core::population::House,
) {
    let persons = person_by_id(world);
    let houses = house_by_id(world);
    let realm_identities = realm_identity_by_id(world);
    let house_identities = house_identity_by_id(world);
    let rr = world
        .family_world
        .rights_world
        .rights
        .realms
        .iter()
        .find(|r| r.realm_id == realm_id)
        .expect("realm rights");
    let incumbent = persons
        .get(rr.incumbent_person_id.as_str())
        .copied()
        .expect("incumbent");
    let h0 = houses
        .get(incumbent.house_id.as_str())
        .copied()
        .expect("H0");
    let ri = realm_identities
        .get(realm_id)
        .copied()
        .expect("realm identity");
    let mut h1 = None;
    let mut h2 = None;
    for house in world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population
        .houses
        .iter()
        .filter(|h| h.realm_id == realm_id)
    {
        if house.id == h0.id {
            continue;
        }
        let hi = house_identities
            .get(house.id.as_str())
            .copied()
            .expect("house identity");
        if hi.culture_id == ri.majority_culture_id && hi.religion_id != ri.majority_religion_id {
            h1 = Some(house);
        } else if hi.culture_id != ri.majority_culture_id
            && hi.religion_id == ri.majority_religion_id
        {
            h2 = Some(house);
        }
    }
    (h0, h1.expect("H1"), h2.expect("H2"))
}

fn candidate_of(
    transition: &SuccessionTransition,
    priority: SuccessionPriority,
) -> &epoch_core::SuccessionCandidate {
    transition
        .candidates
        .iter()
        .find(|c| c.priority == priority)
        .expect("candidate of priority")
}

fn parent_id_set(ids: &[String]) -> BTreeSet<&str> {
    ids.iter().map(String::as_str).collect()
}

#[test]
fn seed1_realm01_structural_trace() {
    let world = generate_succession_world(1, "realm-01").expect("succession");
    let pre = &world.pre_succession_world;
    let persons = person_by_id(pre);
    let supporting = supporting_ids(pre);
    let actors = active_by_person(pre);
    let (h0, _h1, h2) = classify_realm_houses(pre, "realm-01");
    let rights = pre
        .family_world
        .rights_world
        .rights
        .realms
        .iter()
        .find(|r| r.realm_id == "realm-01")
        .expect("realm-01 rights");
    let incumbent = persons
        .get(rights.incumbent_person_id.as_str())
        .copied()
        .expect("incumbent");
    let t = &world.transition;

    assert_eq!(world.schema_version, SUCCESSION_WORLD_SCHEMA_VERSION);
    assert_eq!(world.seed, 1);
    assert_eq!(t.realm_id, "realm-01");
    assert_eq!(t.succession_target_key, "succession:realm-01");
    assert_eq!(t.death.id, "death:realm-01:incumbent");
    assert_eq!(t.death.realm_id, "realm-01");
    assert_eq!(t.death.person_id, incumbent.id);
    assert!(t.vacancy.is_vacant);
    assert_eq!(t.vacancy.former_incumbent_person_id, incumbent.id);
    assert_eq!(t.candidates.len(), SUCCESSION_CANDIDATE_COUNT);

    let a = candidate_of(t, SuccessionPriority::DirectStrongOriginal);
    let b = candidate_of(t, SuccessionPriority::RestoredContestedOriginal);
    let c = candidate_of(t, SuccessionPriority::RestoredContestedDerived);
    let direct = pre
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .find(|claim| claim.realm_id == "realm-01" && claim.basis == ClaimBasis::DirectDescent)
        .expect("direct");
    let restored = pre
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .find(|claim| claim.realm_id == "realm-01" && claim.basis == ClaimBasis::RestoredLineRecord)
        .expect("restored");
    let derived = pre
        .propagation
        .derived_claims
        .iter()
        .find(|claim| claim.realm_id == "realm-01")
        .expect("derived");

    let person_a = persons.get(a.person_id.as_str()).copied().expect("A");
    assert_eq!(a.claim_origin, SuccessionClaimOrigin::Original);
    assert_eq!(a.generation_distance, 0);
    assert_eq!(a.claim_record_id, direct.id);
    assert_eq!(direct.standing, ClaimStanding::Strong);
    assert_eq!(person_a.generation, GenerationBand::Young);
    assert_eq!(a.house_id, h0.id);
    assert!(
        person_a
            .known_parent_ids
            .iter()
            .any(|id| id == &incumbent.id)
    );
    assert!(supporting.contains(person_a.id.as_str()));

    let person_b = persons.get(b.person_id.as_str()).copied().expect("B");
    assert_eq!(b.claim_origin, SuccessionClaimOrigin::Original);
    assert_eq!(b.generation_distance, 0);
    assert_eq!(b.claim_record_id, restored.id);
    assert_eq!(restored.standing, ClaimStanding::Contested);
    assert_eq!(person_b.id, h2.head_person_id);
    assert_eq!(b.house_id, h2.id);
    let actor_b = actors.get(person_b.id.as_str()).copied().expect("B actor");
    assert_eq!(actor_b.primary_role, ActiveRole::HouseHead);

    let person_c = persons.get(c.person_id.as_str()).copied().expect("C");
    assert_eq!(c.claim_origin, SuccessionClaimOrigin::Derived);
    assert_eq!(c.generation_distance, DERIVED_GENERATION_DISTANCE);
    assert_eq!(c.claim_record_id, derived.id);
    assert_eq!(derived.source_claim_id, restored.id);
    assert_eq!(c.house_id, h2.id);
    assert_eq!(person_c.generation, GenerationBand::Young);
    assert!(supporting.contains(person_c.id.as_str()));
    let parents = effective_parent_ids(
        &pre.family_world.rights_world,
        &pre.family_world.family,
        person_c.id.as_str(),
    )
    .expect("effective");
    assert!(parent_id_set(&parents).contains(person_b.id.as_str()));

    let ids = BTreeSet::from([
        a.person_id.as_str(),
        b.person_id.as_str(),
        c.person_id.as_str(),
    ]);
    assert_eq!(ids.len(), 3);
    assert!(!ids.contains(incumbent.id.as_str()));
    assert_eq!(t.presumptive_successor_person_id, a.person_id);
    assert_eq!(t.presumptive_successor_house_id, a.house_id);
    assert_eq!(t.candidates.len(), 3);
}

#[test]
fn six_realms_for_fixed_seeds() {
    for seed in SEEDS {
        let pre = generate_claim_propagation_world(seed).expect("pre");
        let mut deaths = 0usize;
        let mut candidates = 0usize;
        let mut winners = 0usize;
        let mut vacancies = 0usize;
        let mut direct_winners = 0usize;
        for realm in &pre.family_world.rights_world.rights.realms {
            let world =
                generate_succession_world(seed, realm.realm_id.as_str()).expect("succession");
            assert_eq!(world.pre_succession_world, pre, "seed={seed}");
            assert_eq!(
                world
                    .pre_succession_world
                    .family_world
                    .rights_world
                    .rights
                    .realms
                    .iter()
                    .find(|r| r.realm_id == realm.realm_id)
                    .expect("rights")
                    .incumbent_person_id,
                realm.incumbent_person_id,
                "seed={seed}"
            );
            let t = &world.transition;
            assert_eq!(t.death.person_id, realm.incumbent_person_id, "seed={seed}");
            assert_eq!(
                t.candidates.len(),
                SUCCESSION_CANDIDATE_COUNT,
                "seed={seed}"
            );
            assert!(t.vacancy.is_vacant, "seed={seed}");
            deaths += 1;
            candidates += t.candidates.len();
            vacancies += 1;
            let winner = candidate_of(t, SuccessionPriority::DirectStrongOriginal);
            assert_eq!(
                t.presumptive_successor_person_id, winner.person_id,
                "seed={seed}"
            );
            winners += 1;
            direct_winners += 1;
            assert_eq!(
                t.candidates
                    .iter()
                    .filter(|c| c.priority == SuccessionPriority::RestoredContestedOriginal)
                    .count(),
                1,
                "seed={seed}"
            );
            assert_eq!(
                t.candidates
                    .iter()
                    .filter(|c| c.priority == SuccessionPriority::RestoredContestedDerived)
                    .count(),
                1,
                "seed={seed}"
            );
        }
        assert_eq!(deaths, 6, "seed={seed}");
        assert_eq!(candidates, 18, "seed={seed}");
        assert_eq!(winners, 6, "seed={seed}");
        assert_eq!(vacancies, 6, "seed={seed}");
        assert_eq!(direct_winners, 6, "seed={seed}");
    }
}

#[test]
fn same_seed_realm_equality_and_bytes() {
    let a = generate_succession_world(1, "realm-01").expect("a");
    let b = generate_succession_world(1, "realm-01").expect("b");
    assert_eq!(a, b);
    assert_eq!(
        a.to_compact_json_bytes().unwrap(),
        b.to_compact_json_bytes().unwrap()
    );
    assert_eq!(a.to_compact_json_bytes().unwrap().len(), 71915);
    let c = generate_succession_world(2, "realm-01").expect("c");
    assert_ne!(
        a.to_compact_json_bytes().unwrap(),
        c.to_compact_json_bytes().unwrap()
    );
    assert_eq!(c.to_compact_json_bytes().unwrap().len(), 71914);
}

#[test]
fn nested_pre_succession_world_not_mutated() {
    for seed in SEEDS {
        let family = generate_family_world(seed).expect("family");
        let rights = generate_rights_world(seed).expect("rights");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        let pre = generate_claim_propagation_world(seed).expect("pre");
        let world = generate_succession_world(seed, "realm-01").expect("succession");
        assert_eq!(world.pre_succession_world, pre, "seed={seed}");
        assert_eq!(
            world.pre_succession_world.to_compact_json_bytes().unwrap(),
            pre.to_compact_json_bytes().unwrap(),
            "seed={seed}"
        );
        assert_eq!(
            world.pre_succession_world.family_world, family,
            "seed={seed}"
        );
        assert_eq!(
            world.pre_succession_world.family_world.rights_world, rights,
            "seed={seed}"
        );
        assert_eq!(
            world
                .pre_succession_world
                .family_world
                .rights_world
                .context_world
                .political
                .dynastic
                .population
                .generation
                .rng_draws,
            dynastic.population.generation.rng_draws,
            "seed={seed}"
        );
        assert_eq!(
            world
                .pre_succession_world
                .family_world
                .rights_world
                .context_world
                .political
                .dynastic
                .world
                .generation
                .rng_draws,
            dynastic.world.generation.rng_draws,
            "seed={seed}"
        );
        for realm in &world
            .pre_succession_world
            .family_world
            .rights_world
            .rights
            .realms
        {
            let original = rights
                .rights
                .realms
                .iter()
                .find(|r| r.realm_id == realm.realm_id)
                .expect("original rights");
            assert_eq!(
                realm.incumbent_person_id, original.incumbent_person_id,
                "seed={seed}"
            );
        }
        assert_eq!(
            world
                .pre_succession_world
                .family_world
                .rights_world
                .context_world
                .political
                .dynastic
                .population
                .ruler_links,
            dynastic.population.ruler_links,
            "seed={seed}"
        );
        assert_eq!(
            world
                .pre_succession_world
                .family_world
                .rights_world
                .context_world
                .political
                .dynastic
                .world
                .rulers,
            dynastic.world.rulers,
            "seed={seed}"
        );
    }
}

#[test]
fn existing_schema_versions_unchanged() {
    assert_eq!(WORLD_SCHEMA_VERSION, 1);
    assert_eq!(DYNASTIC_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(POLITICAL_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(CONTEXT_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(SAVE_SCHEMA_VERSION, 1);
    assert_eq!(RIGHTS_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(FAMILY_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(SUCCESSION_WORLD_SCHEMA_VERSION, 1);
}

#[test]
fn resolve_unknown_realm_fails_closed() {
    let pre = generate_claim_propagation_world(1).expect("pre");
    let err = resolve_incumbent_death(&pre, "realm-99").unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );
}

#[test]
fn malformed_transition_fail_closed_no_panic() {
    let world = generate_succession_world(1, "realm-01").expect("succession");
    let pre = &world.pre_succession_world;
    let ok = world.transition.clone();

    let err = resolve_incumbent_death(pre, "realm-99").unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut death_mismatch = ok.clone();
    death_mismatch.death.person_id = candidate_of(&ok, SuccessionPriority::DirectStrongOriginal)
        .person_id
        .clone();
    let err = validate_succession_transition(pre, &death_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut vacancy_false = ok.clone();
    vacancy_false.vacancy.is_vacant = false;
    let err = validate_succession_transition(pre, &vacancy_false).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut missing_candidate = ok.clone();
    missing_candidate.candidates.pop();
    let err = validate_succession_transition(pre, &missing_candidate).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut dup = ok.clone();
    if let (Some(first), Some(last)) = (dup.candidates.first().cloned(), dup.candidates.last_mut())
    {
        last.person_id = first.person_id;
        last.claim_record_id = first.claim_record_id;
    }
    let err = validate_succession_transition(pre, &dup).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut unknown_person = ok.clone();
    if let Some(first) = unknown_person.candidates.first_mut() {
        first.person_id = "person-does-not-exist".to_string();
    }
    let err = validate_succession_transition(pre, &unknown_person).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut wrong_realm = ok.clone();
    if let Some(first) = wrong_realm.candidates.first_mut() {
        first.person_id = pre
            .family_world
            .rights_world
            .rights
            .realms
            .iter()
            .find(|r| r.realm_id == "realm-02")
            .expect("realm-02")
            .incumbent_person_id
            .clone();
    }
    let err = validate_succession_transition(pre, &wrong_realm).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut wrong_target = ok.clone();
    wrong_target.succession_target_key = "succession:realm-02".to_string();
    let err = validate_succession_transition(pre, &wrong_target).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut bad_origin = ok.clone();
    if let Some(first) = bad_origin.candidates.first_mut() {
        first.claim_origin = SuccessionClaimOrigin::Derived;
    }
    let err = validate_succession_transition(pre, &bad_origin).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut bad_distance = ok.clone();
    if let Some(first) = bad_distance.candidates.first_mut() {
        first.generation_distance = 2;
    }
    let err = validate_succession_transition(pre, &bad_distance).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut bad_priority = ok.clone();
    if let Some(first) = bad_priority.candidates.first_mut() {
        first.priority = SuccessionPriority::RestoredContestedDerived;
    }
    let err = validate_succession_transition(pre, &bad_priority).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut deceased_candidate = ok.clone();
    if let Some(first) = deceased_candidate.candidates.first_mut() {
        first.person_id = ok.death.person_id.clone();
    }
    let err = validate_succession_transition(pre, &deceased_candidate).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut successor_missing = ok.clone();
    successor_missing.presumptive_successor_person_id = "person-does-not-exist".to_string();
    let err = validate_succession_transition(pre, &successor_missing).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut successor_not_direct = ok.clone();
    let restored = candidate_of(&ok, SuccessionPriority::RestoredContestedOriginal);
    successor_not_direct.presumptive_successor_person_id = restored.person_id.clone();
    successor_not_direct.presumptive_successor_house_id = restored.house_id.clone();
    let err = validate_succession_transition(pre, &successor_not_direct).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut unsorted = ok.clone();
    if let (Some(first), Some(second)) = (
        unsorted.candidates.first().cloned(),
        unsorted.candidates.get(1).cloned(),
    ) {
        if let Some(slot0) = unsorted.candidates.first_mut() {
            *slot0 = second;
        }
        if let Some(slot1) = unsorted.candidates.get_mut(1) {
            *slot1 = first;
        }
    }
    let err = validate_succession_transition(pre, &unsorted).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut derived_wrong_source = ok.clone();
    if let Some(derived) = derived_wrong_source
        .candidates
        .iter_mut()
        .find(|c| c.priority == SuccessionPriority::RestoredContestedDerived)
    {
        derived.claim_record_id = candidate_of(&ok, SuccessionPriority::DirectStrongOriginal)
            .claim_record_id
            .clone();
    }
    let err = validate_succession_transition(pre, &derived_wrong_source).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    let mut unknown_deceased = ok.clone();
    unknown_deceased.death.person_id = "person-does-not-exist".to_string();
    let err = validate_succession_transition(pre, &unknown_deceased).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidSuccession(_)),
        "got {err:?}"
    );

    validate_succession_transition(pre, &ok).expect("ok transition");
}

#[test]
fn cli_succession_1_realm01_matches_generated_world() {
    let expected = generate_succession_world(1, "realm-01").expect("expected");
    common::assert_cli_json_eq(&["succession", "1", "realm-01"], &expected);
}

#[test]
fn cli_succession_check_1_and_2_print_exact_ok() {
    common::assert_cli_exact(&["succession-check", "1"], SUCCESSION_CHECK_1);
    common::assert_cli_exact(&["succession-check", "2"], SUCCESSION_CHECK_2);
}

#[test]
fn existing_m0_to_m22_exact_regression() {
    common::assert_cli_exact_regression();
}

#[test]
fn succession_world_deserializes_as_succession_world() {
    let world = generate_succession_world(1, "realm-01").expect("world");
    let pretty = world.to_pretty_json().expect("pretty");
    let parsed: SuccessionWorld = serde_json::from_str(&pretty).expect("parse");
    assert_eq!(parsed, world);
}
