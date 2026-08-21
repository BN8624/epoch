// M2.5 제한된 출생·다음 세대 권리 통합 테스트

use epoch_core::{
    ActiveRole, BIRTH_COUNT, CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION, CONTEXT_WORLD_SCHEMA_VERSION,
    ClaimBasis, ClaimStanding, CoreError, DYNASTIC_WORLD_SCHEMA_VERSION,
    FAMILY_WORLD_SCHEMA_VERSION, GENERATION_CONTINUATION_WORLD_SCHEMA_VERSION, GenerationBand,
    GenerationContinuationWorld, NEWBORN_COUNT, NEXT_GENERATION_CLAIM_COUNT, PERSON_COUNT,
    POLITICAL_WORLD_SCHEMA_VERSION, RIGHTS_WORLD_SCHEMA_VERSION, SAVE_SCHEMA_VERSION,
    SUCCESSION_CANDIDATE_COUNT, SUCCESSION_CLAIM_COUNT, WORLD_SCHEMA_VERSION,
    derive_generation_continuation, generate_claim_propagation_world, generate_dynastic_world,
    generate_family_world, generate_generation_continuation_world, generate_rights_world,
    generate_succession_world, validate_generation_continuation,
};
use std::collections::{BTreeMap, BTreeSet};

mod common;

const SEEDS: [u64; 5] = [0, 1, 2, 42, u64::MAX];
const CONTINUATION_CHECK_1: &str = "CONTINUATION_OK seed=1 births=6 newborns=6 new_claims=6 restored_sources=6 direct_sources=0 population_persons=144 continuation_persons=150 bytes=74557";
const CONTINUATION_CHECK_2: &str = "CONTINUATION_OK seed=2 births=6 newborns=6 new_claims=6 restored_sources=6 direct_sources=0 population_persons=144 continuation_persons=150 bytes=74553";

fn population(world: &GenerationContinuationWorld) -> &epoch_core::population::PopulationSkeleton {
    &world
        .base_world
        .family_world
        .rights_world
        .context_world
        .political
        .dynastic
        .population
}

fn person_by_id(
    world: &GenerationContinuationWorld,
) -> BTreeMap<&str, &epoch_core::population::Person> {
    population(world)
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect()
}

fn house_by_id(
    world: &GenerationContinuationWorld,
) -> BTreeMap<&str, &epoch_core::population::House> {
    population(world)
        .houses
        .iter()
        .map(|h| (h.id.as_str(), h))
        .collect()
}

fn house_identity_by_id(
    world: &GenerationContinuationWorld,
) -> BTreeMap<&str, &epoch_core::HouseIdentity> {
    world
        .base_world
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
    world: &GenerationContinuationWorld,
) -> BTreeMap<&str, &epoch_core::RealmIdentity> {
    world
        .base_world
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
    world: &GenerationContinuationWorld,
) -> BTreeMap<&str, &epoch_core::ActiveActor> {
    world
        .base_world
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

fn classify_realm_houses<'a>(
    world: &'a GenerationContinuationWorld,
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
        .base_world
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
    for house in population(world)
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

fn restored_claim_for_realm<'a>(
    world: &'a GenerationContinuationWorld,
    realm_id: &str,
) -> &'a epoch_core::SuccessionClaim {
    world
        .base_world
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .find(|c| c.realm_id == realm_id && c.basis == ClaimBasis::RestoredLineRecord)
        .expect("restored claim")
}

fn derived_for_realm<'a>(
    world: &'a GenerationContinuationWorld,
    realm_id: &str,
) -> &'a epoch_core::DerivedSuccessionClaim {
    world
        .base_world
        .propagation
        .derived_claims
        .iter()
        .find(|c| c.realm_id == realm_id)
        .expect("derived claim")
}

fn marriage_for_spouses<'a>(
    world: &'a GenerationContinuationWorld,
    left: &str,
    right: &str,
) -> &'a epoch_core::Marriage {
    world
        .base_world
        .family_world
        .family
        .marriages
        .iter()
        .find(|m| {
            let spouses = parent_id_set(&m.spouse_person_ids);
            spouses.len() == 2 && spouses.contains(left) && spouses.contains(right)
        })
        .expect("marriage by spouse set")
}

fn parent_id_set(ids: &[String]) -> BTreeSet<&str> {
    ids.iter().map(String::as_str).collect()
}

fn h0_current_person<'a>(
    world: &'a GenerationContinuationWorld,
    realm_id: &str,
    h0: &epoch_core::population::House,
) -> &'a epoch_core::population::Person {
    let actors = active_by_person(world);
    let persons = person_by_id(world);
    let actor = actors
        .values()
        .copied()
        .find(|a| a.realm_id == realm_id && a.primary_role == ActiveRole::RulingHouseCurrent)
        .expect("H0 current actor");
    let person = persons
        .get(actor.person_id.as_str())
        .copied()
        .expect("H0 current person");
    assert_eq!(person.house_id, h0.id);
    person
}

#[test]
fn counts_sources_and_claimant_shape() {
    for seed in SEEDS {
        let world = generate_generation_continuation_world(seed).expect("continuation");
        assert_eq!(
            world.schema_version,
            GENERATION_CONTINUATION_WORLD_SCHEMA_VERSION
        );
        assert_eq!(world.seed, seed);
        assert_eq!(
            population(&world).persons.len(),
            PERSON_COUNT,
            "seed={seed}"
        );
        assert_eq!(population(&world).houses.len(), 18, "seed={seed}");
        assert_eq!(
            world
                .base_world
                .family_world
                .rights_world
                .rights
                .claims
                .len(),
            SUCCESSION_CLAIM_COUNT,
            "seed={seed}"
        );
        assert_eq!(
            world.base_world.propagation.derived_claims.len(),
            6,
            "seed={seed}"
        );
        assert_eq!(world.continuation.births.len(), BIRTH_COUNT, "seed={seed}");
        assert_eq!(
            world.continuation.newborns.len(),
            NEWBORN_COUNT,
            "seed={seed}"
        );
        assert_eq!(
            world.continuation.derived_claims.len(),
            NEXT_GENERATION_CLAIM_COUNT,
            "seed={seed}"
        );

        let persons = person_by_id(&world);
        let house_identities = house_identity_by_id(&world);
        let mut birth_ids = BTreeSet::new();
        let mut newborn_ids = BTreeSet::new();
        let mut claim_ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut restored_sources = 0usize;
        let mut direct_sources = 0usize;
        let mut marriage_b = 0usize;
        let mut distance1 = 0usize;
        let mut by_realm: BTreeMap<&str, usize> = BTreeMap::new();

        let mut realms: Vec<_> = world
            .base_world
            .family_world
            .rights_world
            .rights
            .realms
            .iter()
            .collect();
        realms.sort_by(|a, b| a.realm_id.cmp(&b.realm_id));
        for (idx, realm) in realms.into_iter().enumerate() {
            let (h0, h1, h2) = classify_realm_houses(&world, realm.realm_id.as_str());
            let h0_current = h0_current_person(&world, realm.realm_id.as_str(), h0);
            let marriage_a_rec = marriage_for_spouses(
                &world,
                h0.head_person_id.as_str(),
                h1.head_person_id.as_str(),
            );
            let marriage_b_rec =
                marriage_for_spouses(&world, h0_current.id.as_str(), h2.head_person_id.as_str());
            let birth = world
                .continuation
                .births
                .iter()
                .find(|b| b.realm_id == realm.realm_id)
                .expect("birth");
            let newborn = world
                .continuation
                .newborns
                .iter()
                .find(|n| n.realm_id == realm.realm_id)
                .expect("newborn");
            let claim = world
                .continuation
                .derived_claims
                .iter()
                .find(|c| c.realm_id == realm.realm_id)
                .expect("next claim");
            let restored = restored_claim_for_realm(&world, realm.realm_id.as_str());
            let candidate_c = derived_for_realm(&world, realm.realm_id.as_str());
            let expected_newborn_id = format!("person-{:03}", PERSON_COUNT + idx + 1);

            assert!(birth_ids.insert(birth.id.as_str()), "seed={seed}");
            assert!(newborn_ids.insert(newborn.id.as_str()), "seed={seed}");
            assert!(claim_ids.insert(claim.id.as_str()), "seed={seed}");
            assert_eq!(birth.id, format!("birth-{:02}", idx + 1), "seed={seed}");
            assert_eq!(
                claim.id,
                format!("next-claim-{:02}", idx + 1),
                "seed={seed}"
            );
            assert_eq!(newborn.id, expected_newborn_id, "seed={seed}");
            assert_eq!(birth.child_person_id, newborn.id, "seed={seed}");
            assert_eq!(claim.claimant_person_id, newborn.id, "seed={seed}");
            assert_eq!(birth.marriage_id, marriage_b_rec.id, "seed={seed}");
            assert_ne!(birth.marriage_id, marriage_a_rec.id, "seed={seed}");
            marriage_b += 1;
            assert_eq!(
                parent_id_set(&birth.parent_person_ids),
                parent_id_set(&marriage_b_rec.spouse_person_ids),
                "seed={seed}"
            );
            assert_eq!(birth.parent_person_ids.len(), 2, "seed={seed}");
            for parent_id in &birth.parent_person_ids {
                let parent = persons.get(parent_id.as_str()).copied().expect("parent");
                assert_eq!(parent.generation, GenerationBand::Current, "seed={seed}");
                assert_ne!(parent.id, realm.incumbent_person_id, "seed={seed}");
                assert!(
                    persons.contains_key(parent.id.as_str()),
                    "seed={seed} parent not in 144"
                );
            }
            assert!(
                !persons.contains_key(newborn.id.as_str()),
                "seed={seed} newborn collides"
            );
            assert_eq!(newborn.house_id, h2.id, "seed={seed}");
            assert_eq!(newborn.realm_id, h2.realm_id, "seed={seed}");
            assert_eq!(
                newborn.home_territory_id, h2.seat_territory_id,
                "seed={seed}"
            );
            let hi = house_identities
                .get(h2.id.as_str())
                .copied()
                .expect("H2 identity");
            assert_eq!(newborn.culture_id, hi.culture_id, "seed={seed}");
            assert_eq!(newborn.religion_id, hi.religion_id, "seed={seed}");
            assert!(!newborn.name.is_empty(), "seed={seed}");
            assert_ne!(newborn.name, newborn.id, "seed={seed}");
            assert_eq!(newborn.name, format!("{} 후대 1", h2.name), "seed={seed}");
            assert!(names.insert(newborn.name.as_str()), "seed={seed}");
            assert_eq!(claim.source_claim_id, restored.id, "seed={seed}");
            match restored.basis {
                ClaimBasis::RestoredLineRecord => restored_sources += 1,
                ClaimBasis::DirectDescent => direct_sources += 1,
            }
            assert_eq!(restored.standing, ClaimStanding::Contested, "seed={seed}");
            assert_eq!(
                claim.via_parent_person_id, restored.claimant_person_id,
                "seed={seed}"
            );
            assert_eq!(claim.via_parent_person_id, h2.head_person_id, "seed={seed}");
            assert_eq!(claim.claimant_house_id, h2.id, "seed={seed}");
            assert_eq!(claim.generation_distance, 1, "seed={seed}");
            assert_eq!(
                claim.succession_target_key, restored.succession_target_key,
                "seed={seed}"
            );
            assert_ne!(candidate_c.claimant_person_id, newborn.id, "seed={seed}");
            assert_eq!(
                candidate_c.claimant_house_id, newborn.house_id,
                "seed={seed}"
            );
            assert_eq!(
                candidate_c.source_claim_id, claim.source_claim_id,
                "seed={seed}"
            );
            assert_eq!(
                candidate_c.via_parent_person_id, claim.via_parent_person_id,
                "seed={seed}"
            );
            if claim.generation_distance == 1 {
                distance1 += 1;
            }
            *by_realm.entry(realm.realm_id.as_str()).or_insert(0) += 1;
        }

        assert_eq!(birth_ids.len(), 6, "seed={seed}");
        assert_eq!(newborn_ids.len(), 6, "seed={seed}");
        assert_eq!(claim_ids.len(), 6, "seed={seed}");
        assert_eq!(names.len(), 6, "seed={seed}");
        assert_eq!(marriage_b, 6, "seed={seed}");
        assert_eq!(restored_sources, 6, "seed={seed}");
        assert_eq!(direct_sources, 0, "seed={seed}");
        assert_eq!(distance1, 6, "seed={seed}");
        assert_eq!(by_realm.len(), 6, "seed={seed}");
        for (realm_id, count) in &by_realm {
            assert_eq!(*count, 1, "seed={seed} realm={realm_id}");
        }
        assert_eq!(
            PERSON_COUNT + world.continuation.newborns.len(),
            150,
            "seed={seed}"
        );
    }
}

#[test]
fn same_seed_equality_and_bytes() {
    let a = generate_generation_continuation_world(1).expect("a");
    let b = generate_generation_continuation_world(1).expect("b");
    assert_eq!(a, b);
    assert_eq!(
        a.to_compact_json_bytes().unwrap(),
        b.to_compact_json_bytes().unwrap()
    );
    assert_eq!(a.to_compact_json_bytes().unwrap().len(), 74557);
    let c = generate_generation_continuation_world(2).expect("c");
    assert_ne!(
        a.to_compact_json_bytes().unwrap(),
        c.to_compact_json_bytes().unwrap()
    );
    assert_eq!(c.to_compact_json_bytes().unwrap().len(), 74553);
}

#[test]
fn nested_base_world_not_mutated() {
    for seed in SEEDS {
        let family = generate_family_world(seed).expect("family");
        let rights = generate_rights_world(seed).expect("rights");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        let pre = generate_claim_propagation_world(seed).expect("pre");
        let world = generate_generation_continuation_world(seed).expect("continuation");
        assert_eq!(world.base_world, pre, "seed={seed}");
        assert_eq!(
            world.base_world.to_compact_json_bytes().unwrap(),
            pre.to_compact_json_bytes().unwrap(),
            "seed={seed}"
        );
        assert_eq!(world.base_world.family_world, family, "seed={seed}");
        assert_eq!(
            world.base_world.family_world.rights_world, rights,
            "seed={seed}"
        );
        assert_eq!(
            world
                .base_world
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
                .base_world
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
        assert_eq!(
            world
                .base_world
                .family_world
                .rights_world
                .context_world
                .political
                .dynastic
                .schema_version,
            DYNASTIC_WORLD_SCHEMA_VERSION
        );
        assert_eq!(
            world.base_world.family_world.schema_version,
            FAMILY_WORLD_SCHEMA_VERSION
        );
        assert_eq!(
            world.base_world.family_world.rights_world.schema_version,
            RIGHTS_WORLD_SCHEMA_VERSION
        );
        assert_eq!(
            world
                .base_world
                .family_world
                .rights_world
                .context_world
                .schema_version,
            CONTEXT_WORLD_SCHEMA_VERSION
        );
        assert_eq!(
            world
                .base_world
                .family_world
                .rights_world
                .context_world
                .political
                .schema_version,
            POLITICAL_WORLD_SCHEMA_VERSION
        );
        assert_eq!(
            world
                .base_world
                .family_world
                .rights_world
                .context_world
                .political
                .dynastic
                .world
                .schema_version,
            WORLD_SCHEMA_VERSION
        );
        assert_eq!(
            world.base_world.schema_version,
            CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION
        );
        assert_eq!(SAVE_SCHEMA_VERSION, 1);
        for house in &population(&world).houses {
            for newborn in &world.continuation.newborns {
                assert!(
                    !house.member_ids.iter().any(|id| id == &newborn.id),
                    "seed={seed} house {} gained newborn",
                    house.id
                );
            }
        }
    }
}

#[test]
fn seed1_realm01_structural_trace() {
    let world = generate_generation_continuation_world(1).expect("continuation");
    let persons = person_by_id(&world);
    let actors = active_by_person(&world);
    let (h0, h1, h2) = classify_realm_houses(&world, "realm-01");
    let rights = world
        .base_world
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
    let h0_current = h0_current_person(&world, "realm-01", h0);
    let marriage_b =
        marriage_for_spouses(&world, h0_current.id.as_str(), h2.head_person_id.as_str());
    let marriage_a = marriage_for_spouses(
        &world,
        h0.head_person_id.as_str(),
        h1.head_person_id.as_str(),
    );
    let birth = world
        .continuation
        .births
        .iter()
        .find(|b| b.id == "birth-01")
        .expect("birth-01");
    let newborn = world
        .continuation
        .newborns
        .iter()
        .find(|n| n.id == "person-145")
        .expect("person-145");
    let claim = world
        .continuation
        .derived_claims
        .iter()
        .find(|c| c.id == "next-claim-01")
        .expect("next-claim-01");
    let restored = restored_claim_for_realm(&world, "realm-01");
    let candidate_c = derived_for_realm(&world, "realm-01");

    assert_eq!(incumbent.id, "person-003");
    assert_eq!(h0_current.id, "person-004");
    assert_eq!(h0_current.generation, GenerationBand::Current);
    assert_eq!(
        actors
            .get(h0_current.id.as_str())
            .copied()
            .expect("H0 current actor")
            .primary_role,
        ActiveRole::RulingHouseCurrent
    );
    assert_eq!(h2.head_person_id, "person-019");
    assert_eq!(
        actors
            .get(h2.head_person_id.as_str())
            .copied()
            .expect("H2 head actor")
            .primary_role,
        ActiveRole::HouseHead
    );
    assert_eq!(birth.marriage_id, marriage_b.id);
    assert_ne!(birth.marriage_id, marriage_a.id);
    assert_eq!(
        parent_id_set(&birth.parent_person_ids),
        parent_id_set(&marriage_b.spouse_person_ids)
    );
    assert!(birth.parent_person_ids.iter().any(|id| id == "person-004"));
    assert!(birth.parent_person_ids.iter().any(|id| id == "person-019"));
    assert!(!birth.parent_person_ids.iter().any(|id| id == "person-003"));
    assert_eq!(newborn.id, "person-145");
    assert_eq!(newborn.realm_id, "realm-01");
    assert_eq!(newborn.house_id, h2.id);
    assert_eq!(newborn.house_id, "house-03");
    assert_eq!(newborn.home_territory_id, h2.seat_territory_id);
    assert_eq!(newborn.name, format!("{} 후대 1", h2.name));
    assert_eq!(claim.source_claim_id, restored.id);
    assert_eq!(restored.basis, ClaimBasis::RestoredLineRecord);
    assert_eq!(restored.standing, ClaimStanding::Contested);
    assert_eq!(claim.via_parent_person_id, "person-019");
    assert_eq!(claim.via_parent_person_id, restored.claimant_person_id);
    assert_eq!(claim.claimant_person_id, "person-145");
    assert_eq!(claim.generation_distance, 1);
    assert_eq!(claim.succession_target_key, "succession:realm-01");
    assert_eq!(candidate_c.claimant_person_id, "person-022");
    assert_ne!(candidate_c.claimant_person_id, newborn.id);
    assert_eq!(candidate_c.claimant_house_id, newborn.house_id);
    assert_eq!(candidate_c.source_claim_id, claim.source_claim_id);
    assert_eq!(candidate_c.via_parent_person_id, claim.via_parent_person_id);
}

#[test]
fn current_succession_crisis_unchanged() {
    for seed in SEEDS {
        let world = generate_generation_continuation_world(seed).expect("continuation");
        for realm in &world.base_world.family_world.rights_world.rights.realms {
            let succession =
                generate_succession_world(seed, realm.realm_id.as_str()).expect("succession");
            assert_eq!(
                succession.pre_succession_world, world.base_world,
                "seed={seed}"
            );
            assert_eq!(
                succession.transition.candidates.len(),
                SUCCESSION_CANDIDATE_COUNT,
                "seed={seed}"
            );
            for newborn in &world.continuation.newborns {
                assert!(
                    succession
                        .transition
                        .candidates
                        .iter()
                        .all(|c| c.person_id != newborn.id),
                    "seed={seed} newborn became candidate"
                );
            }
        }
    }
    let seed1 = generate_succession_world(1, "realm-01").expect("seed1");
    assert_eq!(seed1.to_compact_json_bytes().unwrap().len(), 71915);
    assert_eq!(seed1.transition.candidates.len(), 3);
    let seed2 = generate_succession_world(2, "realm-01").expect("seed2");
    assert_eq!(seed2.to_compact_json_bytes().unwrap().len(), 71914);
    assert_eq!(seed2.transition.candidates.len(), 3);
}

#[test]
fn fail_closed_malformed_continuation() {
    let world = generate_generation_continuation_world(1).expect("ok");
    let base = &world.base_world;
    let (h0, h1, _h2) = classify_realm_houses(&world, "realm-01");
    let rights = base
        .family_world
        .rights_world
        .rights
        .realms
        .iter()
        .find(|r| r.realm_id == "realm-01")
        .expect("rights");
    let marriage_a = marriage_for_spouses(
        &world,
        h0.head_person_id.as_str(),
        h1.head_person_id.as_str(),
    );
    let ok = world.continuation.clone();

    let mut missing = ok.clone();
    missing.births.pop();
    let err = validate_generation_continuation(base, &missing).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut missing_newborn = ok.clone();
    missing_newborn.newborns.pop();
    let err = validate_generation_continuation(base, &missing_newborn).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut missing_claim = ok.clone();
    missing_claim.derived_claims.pop();
    let err = validate_generation_continuation(base, &missing_claim).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut dup_birth = ok.clone();
    if let (Some(first), Some(last)) = (
        dup_birth.births.first().cloned(),
        dup_birth.births.last_mut(),
    ) {
        last.id = first.id;
    }
    let err = validate_generation_continuation(base, &dup_birth).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut dup_newborn = ok.clone();
    if let (Some(first), Some(last)) = (
        dup_newborn.newborns.first().cloned(),
        dup_newborn.newborns.last_mut(),
    ) {
        last.id = first.id;
    }
    let err = validate_generation_continuation(base, &dup_newborn).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut dup_claim = ok.clone();
    if let (Some(first), Some(last)) = (
        dup_claim.derived_claims.first().cloned(),
        dup_claim.derived_claims.last_mut(),
    ) {
        last.id = first.id;
    }
    let err = validate_generation_continuation(base, &dup_claim).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut marriage_a_used = ok.clone();
    if let Some(first) = marriage_a_used.births.first_mut() {
        first.marriage_id = marriage_a.id.clone();
        first.parent_person_ids = marriage_a.spouse_person_ids.clone();
    }
    let err = validate_generation_continuation(base, &marriage_a_used).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut unknown_marriage = ok.clone();
    if let Some(first) = unknown_marriage.births.first_mut() {
        first.marriage_id = "marriage-does-not-exist".to_string();
    }
    let err = validate_generation_continuation(base, &unknown_marriage).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut parent_count = ok.clone();
    if let Some(first) = parent_count.births.first_mut() {
        first.parent_person_ids.pop();
    }
    let err = validate_generation_continuation(base, &parent_count).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut incumbent_parent = ok.clone();
    if let Some(first) = incumbent_parent.births.first_mut() {
        if let Some(slot) = first.parent_person_ids.first_mut() {
            *slot = rights.incumbent_person_id.clone();
        }
        first.parent_person_ids.sort();
    }
    let err = validate_generation_continuation(base, &incumbent_parent).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut spouse_mismatch = ok.clone();
    if let Some(first) = spouse_mismatch.births.first_mut() {
        first.parent_person_ids = marriage_a.spouse_person_ids.clone();
    }
    let err = validate_generation_continuation(base, &spouse_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut unknown_parent = ok.clone();
    if let Some(first) = unknown_parent.births.first_mut() {
        if let Some(slot) = first.parent_person_ids.first_mut() {
            *slot = "person-does-not-exist".to_string();
        }
        first.parent_person_ids.sort();
    }
    let err = validate_generation_continuation(base, &unknown_parent).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut collision = ok.clone();
    if let Some(first) = collision.newborns.first_mut() {
        first.id = "person-003".to_string();
    }
    if let Some(first) = collision.births.first_mut() {
        first.child_person_id = "person-003".to_string();
    }
    if let Some(first) = collision.derived_claims.first_mut() {
        first.claimant_person_id = "person-003".to_string();
    }
    let err = validate_generation_continuation(base, &collision).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut candidate_c_collision = ok.clone();
    if let Some(first) = candidate_c_collision.newborns.first_mut() {
        first.id = "person-022".to_string();
    }
    if let Some(first) = candidate_c_collision.births.first_mut() {
        first.child_person_id = "person-022".to_string();
    }
    if let Some(first) = candidate_c_collision.derived_claims.first_mut() {
        first.claimant_person_id = "person-022".to_string();
    }
    let err = validate_generation_continuation(base, &candidate_c_collision).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut wrong_house = ok.clone();
    if let Some(first) = wrong_house.newborns.first_mut() {
        first.house_id = h0.id.clone();
    }
    if let Some(first) = wrong_house.derived_claims.first_mut() {
        first.claimant_house_id = h0.id.clone();
    }
    let err = validate_generation_continuation(base, &wrong_house).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut unknown_realm = ok.clone();
    if let Some(first) = unknown_realm.births.first_mut() {
        first.realm_id = "realm-does-not-exist".to_string();
    }
    let err = validate_generation_continuation(base, &unknown_realm).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut missing_source = ok.clone();
    if let Some(first) = missing_source.derived_claims.first_mut() {
        first.source_claim_id = "claim-does-not-exist".to_string();
    }
    let err = validate_generation_continuation(base, &missing_source).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let direct = base
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .find(|c| c.realm_id == "realm-01" && c.basis == ClaimBasis::DirectDescent)
        .expect("direct");
    let mut not_restored = ok.clone();
    if let Some(first) = not_restored.derived_claims.first_mut() {
        first.source_claim_id = direct.id.clone();
        first.via_parent_person_id = direct.claimant_person_id.clone();
    }
    let err = validate_generation_continuation(base, &not_restored).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut via_mismatch = ok.clone();
    if let Some(first) = via_mismatch.derived_claims.first_mut() {
        first.via_parent_person_id = h0.head_person_id.clone();
    }
    let err = validate_generation_continuation(base, &via_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut claimant_mismatch = ok.clone();
    if let Some(first) = claimant_mismatch.derived_claims.first_mut() {
        first.claimant_person_id = "person-146".to_string();
    }
    let err = validate_generation_continuation(base, &claimant_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut distance_zero = ok.clone();
    if let Some(first) = distance_zero.derived_claims.first_mut() {
        first.generation_distance = 0;
    }
    let err = validate_generation_continuation(base, &distance_zero).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut target_mismatch = ok.clone();
    if let Some(first) = target_mismatch.derived_claims.first_mut() {
        first.succession_target_key = "succession:realm-02".to_string();
    }
    let err = validate_generation_continuation(base, &target_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut bad_schema = base.clone();
    bad_schema.schema_version = 99;
    let err = validate_generation_continuation(&bad_schema, &ok).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let mut bad_seed = base.clone();
    bad_seed.seed = base.seed.wrapping_add(1);
    let err = validate_generation_continuation(&bad_seed, &ok).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidContinuation(_)),
        "got {err:?}"
    );

    let derived = derive_generation_continuation(base).expect("derive");
    assert_eq!(derived.births.len(), BIRTH_COUNT);
}

#[test]
fn cli_continuation_1_matches_generated_world() {
    let expected = generate_generation_continuation_world(1).expect("expected");
    common::assert_cli_json_eq(&["continuation", "1"], &expected);
}

#[test]
fn cli_continuation_check_1_and_2_print_exact_ok() {
    common::assert_cli_exact(&["continuation-check", "1"], CONTINUATION_CHECK_1);
    common::assert_cli_exact(&["continuation-check", "2"], CONTINUATION_CHECK_2);
}

#[test]
fn existing_m0_to_m23_exact_regression() {
    common::assert_cli_exact_regression();
}

#[test]
fn continuation_world_deserializes_as_continuation_world() {
    let world = generate_generation_continuation_world(1).expect("world");
    let pretty = world.to_pretty_json().expect("pretty");
    let parsed: GenerationContinuationWorld = serde_json::from_str(&pretty).expect("parse");
    assert_eq!(parsed, world);
}
