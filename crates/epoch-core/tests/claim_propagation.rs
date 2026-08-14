// M2.2 1세대 권리 전파 — 부모 Claim에서 자녀 파생 권리 통합 테스트

use epoch_core::{
    ActiveRole, CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION, CONTEXT_WORLD_SCHEMA_VERSION, ClaimBasis,
    ClaimPropagationWorld, ClaimStanding, CoreError, DERIVED_CLAIM_COUNT,
    DERIVED_GENERATION_DISTANCE, DYNASTIC_WORLD_SCHEMA_VERSION, DerivedSuccessionClaim,
    FAMILY_WORLD_SCHEMA_VERSION, GenerationBand, InitialClaimPropagation,
    POLITICAL_WORLD_SCHEMA_VERSION, RIGHTS_WORLD_SCHEMA_VERSION, SAVE_SCHEMA_VERSION,
    SUCCESSION_CLAIM_COUNT, WORLD_SCHEMA_VERSION, derive_initial_claim_propagation,
    effective_parent_ids, generate_claim_propagation_world, generate_dynastic_world,
    generate_family_world, generate_rights_world, validate_initial_claim_propagation,
};
use std::collections::{BTreeMap, BTreeSet};

mod common;

const SEEDS: [u64; 5] = [0, 1, 2, 42, u64::MAX];

fn person_by_id(world: &ClaimPropagationWorld) -> BTreeMap<&str, &epoch_core::population::Person> {
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

fn house_by_id(world: &ClaimPropagationWorld) -> BTreeMap<&str, &epoch_core::population::House> {
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
    world: &ClaimPropagationWorld,
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
    world: &ClaimPropagationWorld,
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

fn active_by_person(world: &ClaimPropagationWorld) -> BTreeMap<&str, &epoch_core::ActiveActor> {
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

fn supporting_ids(world: &ClaimPropagationWorld) -> BTreeSet<&str> {
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

fn claim_by_id(world: &ClaimPropagationWorld) -> BTreeMap<&str, &epoch_core::SuccessionClaim> {
    world
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect()
}

fn original_claimants(world: &ClaimPropagationWorld) -> BTreeSet<&str> {
    world
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .map(|c| c.claimant_person_id.as_str())
        .collect()
}

fn classify_realm_houses<'a>(
    world: &'a ClaimPropagationWorld,
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

fn restored_claim_for_realm<'a>(
    world: &'a ClaimPropagationWorld,
    realm_id: &str,
) -> &'a epoch_core::SuccessionClaim {
    world
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .find(|c| c.realm_id == realm_id && c.basis == ClaimBasis::RestoredLineRecord)
        .expect("restored claim")
}

fn direct_claim_for_realm<'a>(
    world: &'a ClaimPropagationWorld,
    realm_id: &str,
) -> &'a epoch_core::SuccessionClaim {
    world
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .find(|c| c.realm_id == realm_id && c.basis == ClaimBasis::DirectDescent)
        .expect("direct claim")
}

fn derived_for_realm<'a>(
    world: &'a ClaimPropagationWorld,
    realm_id: &str,
) -> &'a DerivedSuccessionClaim {
    world
        .propagation
        .derived_claims
        .iter()
        .find(|c| c.realm_id == realm_id)
        .expect("derived claim")
}

fn parent_id_set(link_parents: &[String]) -> BTreeSet<&str> {
    link_parents.iter().map(String::as_str).collect()
}

fn marriage_containing<'a>(
    world: &'a ClaimPropagationWorld,
    person_id: &str,
) -> &'a epoch_core::Marriage {
    world
        .family_world
        .family
        .marriages
        .iter()
        .find(|m| m.spouse_person_ids.iter().any(|id| id == person_id))
        .expect("marriage")
}

fn expected_check_line(seed: u64) -> String {
    let world = generate_claim_propagation_world(seed).expect("world");
    let bytes = world.to_compact_json_bytes().expect("bytes").len();
    format!(
        "CLAIM_PROPAGATION_OK seed={seed} original=12 derived=6 restored_sources=6 direct_sources=0 distance1=6 derived_supporting=6 bytes={bytes}"
    )
}

#[test]
fn counts_sources_and_claimant_shape() {
    for seed in SEEDS {
        let world = generate_claim_propagation_world(seed).expect("propagation");
        assert_eq!(world.schema_version, CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION);
        assert_eq!(world.seed, seed);
        assert_eq!(
            world.family_world.rights_world.rights.claims.len(),
            SUCCESSION_CLAIM_COUNT,
            "seed={seed}"
        );
        assert_eq!(
            world.propagation.derived_claims.len(),
            DERIVED_CLAIM_COUNT,
            "seed={seed}"
        );

        let persons = person_by_id(&world);
        let claims = claim_by_id(&world);
        let actors = active_by_person(&world);
        let supporting = supporting_ids(&world);
        let originals = original_claimants(&world);
        let mut derived_ids = BTreeSet::new();
        let mut derived_claimants = BTreeSet::new();
        let mut source_ids = BTreeSet::new();
        let mut restored_sources = 0usize;
        let mut direct_sources = 0usize;
        let mut distance1 = 0usize;
        let mut young = 0usize;
        let mut derived_supporting = 0usize;
        let mut derived_active = 0usize;
        let mut by_realm: BTreeMap<&str, usize> = BTreeMap::new();

        for derived in &world.propagation.derived_claims {
            assert!(
                derived_ids.insert(derived.id.as_str()),
                "seed={seed} duplicate derived {}",
                derived.id
            );
            assert!(
                derived_claimants.insert(derived.claimant_person_id.as_str()),
                "seed={seed} duplicate claimant {}",
                derived.claimant_person_id
            );
            assert!(
                source_ids.insert(derived.source_claim_id.as_str()),
                "seed={seed} duplicate source {}",
                derived.source_claim_id
            );
            let source = claims
                .get(derived.source_claim_id.as_str())
                .copied()
                .expect("source");
            match source.basis {
                ClaimBasis::RestoredLineRecord => restored_sources += 1,
                ClaimBasis::DirectDescent => direct_sources += 1,
            }
            assert_eq!(source.standing, ClaimStanding::Contested, "seed={seed}");
            assert_eq!(
                source.claimant_person_id, derived.via_parent_person_id,
                "seed={seed}"
            );
            assert_eq!(derived.realm_id, source.realm_id, "seed={seed}");
            assert_eq!(
                derived.succession_target_key, source.succession_target_key,
                "seed={seed}"
            );
            if derived.generation_distance == DERIVED_GENERATION_DISTANCE {
                distance1 += 1;
            }
            let child = persons
                .get(derived.claimant_person_id.as_str())
                .copied()
                .expect("child");
            let parent = persons
                .get(derived.via_parent_person_id.as_str())
                .copied()
                .expect("parent");
            assert_eq!(child.generation, GenerationBand::Young, "seed={seed}");
            young += 1;
            assert_eq!(child.house_id, derived.claimant_house_id, "seed={seed}");
            assert_eq!(child.realm_id, derived.realm_id, "seed={seed}");
            assert_eq!(parent.realm_id, derived.realm_id, "seed={seed}");
            if supporting.contains(child.id.as_str()) {
                derived_supporting += 1;
            }
            if actors.contains_key(child.id.as_str()) {
                derived_active += 1;
            }
            let actor = actors
                .get(parent.id.as_str())
                .copied()
                .expect("source actor");
            assert_eq!(actor.primary_role, ActiveRole::HouseHead, "seed={seed}");
            let parents = effective_parent_ids(
                &world.family_world.rights_world,
                &world.family_world.family,
                child.id.as_str(),
            )
            .expect("effective parents");
            assert!(
                parents.iter().any(|id| id == &parent.id),
                "seed={seed} missing parentage path"
            );
            *by_realm.entry(derived.realm_id.as_str()).or_insert(0) += 1;
        }

        assert_eq!(derived_ids.len(), 6, "seed={seed}");
        assert_eq!(derived_claimants.len(), 6, "seed={seed}");
        assert_eq!(source_ids.len(), 6, "seed={seed}");
        assert_eq!(restored_sources, 6, "seed={seed}");
        assert_eq!(direct_sources, 0, "seed={seed}");
        assert_eq!(distance1, 6, "seed={seed}");
        assert_eq!(young, 6, "seed={seed}");
        assert_eq!(derived_supporting, 6, "seed={seed}");
        assert_eq!(derived_active, 0, "seed={seed}");
        assert_eq!(by_realm.len(), 6, "seed={seed}");
        for (realm_id, count) in &by_realm {
            assert_eq!(*count, 1, "seed={seed} realm={realm_id}");
        }
        assert_eq!(
            derived_claimants.intersection(&originals).count(),
            0,
            "seed={seed}"
        );
    }
}

#[test]
fn same_seed_equality_and_bytes() {
    let a = generate_claim_propagation_world(1).expect("a");
    let b = generate_claim_propagation_world(1).expect("b");
    assert_eq!(a, b);
    assert_eq!(
        a.to_compact_json_bytes().unwrap(),
        b.to_compact_json_bytes().unwrap()
    );
    let c = generate_claim_propagation_world(2).expect("c");
    assert_ne!(
        a.to_compact_json_bytes().unwrap(),
        c.to_compact_json_bytes().unwrap()
    );
    assert_eq!(c.propagation.derived_claims.len(), DERIVED_CLAIM_COUNT);
}

#[test]
fn lower_layer_bytes_and_rng_unchanged() {
    for seed in SEEDS {
        let family = generate_family_world(seed).expect("family");
        let rights = generate_rights_world(seed).expect("rights");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        let world = generate_claim_propagation_world(seed).expect("propagation");
        assert_eq!(world.family_world, family, "seed={seed}");
        assert_eq!(
            world.family_world.to_compact_json_bytes().unwrap(),
            family.to_compact_json_bytes().unwrap(),
            "seed={seed}"
        );
        assert_eq!(world.family_world.rights_world, rights, "seed={seed}");
        assert_eq!(
            world
                .family_world
                .rights_world
                .to_compact_json_bytes()
                .unwrap(),
            rights.to_compact_json_bytes().unwrap(),
            "seed={seed}"
        );
        assert_eq!(
            world
                .family_world
                .rights_world
                .context_world
                .political
                .dynastic
                .population,
            dynastic.population,
            "seed={seed}"
        );
        assert_eq!(
            world
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
            world.family_world.rights_world.rights.claims.len(),
            SUCCESSION_CLAIM_COUNT,
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
}

#[test]
fn seed1_realm01_structural_trace() {
    let world = generate_claim_propagation_world(1).expect("propagation");
    let persons = person_by_id(&world);
    let (h0, _h1, h2) = classify_realm_houses(&world, "realm-01");
    let restored = restored_claim_for_realm(&world, "realm-01");
    let actors = active_by_person(&world);
    let supporting = supporting_ids(&world);

    assert_eq!(restored.basis, ClaimBasis::RestoredLineRecord);
    assert_eq!(restored.standing, ClaimStanding::Contested);
    assert_eq!(restored.claimant_person_id, h2.head_person_id);
    let source_actor = actors
        .get(restored.claimant_person_id.as_str())
        .copied()
        .expect("restored actor");
    assert_eq!(source_actor.primary_role, ActiveRole::HouseHead);

    let h0_current = h0.member_ids.get(3).expect("H0[3]");
    let h2_child = h2.member_ids.get(5).expect("H2[5]");
    let marriage_b = world
        .family_world
        .family
        .marriages
        .iter()
        .find(|m| m.id == "marriage-02")
        .expect("Marriage B");
    let b_spouses = parent_id_set(&marriage_b.spouse_person_ids);
    assert!(b_spouses.contains(h2.head_person_id.as_str()));
    assert!(b_spouses.contains(h0_current.as_str()));

    let parentage_b = world
        .family_world
        .family
        .parentages
        .iter()
        .find(|p| p.id == "parentage-02")
        .expect("parentage B");
    assert_eq!(&parentage_b.child_person_id, h2_child);
    let parents = effective_parent_ids(
        &world.family_world.rights_world,
        &world.family_world.family,
        h2_child,
    )
    .expect("effective");
    assert_eq!(parents.len(), 2);
    let parent_set = parent_id_set(&parents);
    assert!(parent_set.contains(h2.head_person_id.as_str()));
    assert!(parent_set.contains(h0_current.as_str()));
    assert!(parent_set.contains(restored.claimant_person_id.as_str()));

    let derived = world
        .propagation
        .derived_claims
        .iter()
        .find(|c| c.id == "derived-claim-01")
        .expect("derived-claim-01");
    assert_eq!(derived.source_claim_id, restored.id);
    assert_eq!(derived.via_parent_person_id, restored.claimant_person_id);
    assert_eq!(&derived.claimant_person_id, h2_child);
    assert_eq!(derived.claimant_house_id, h2.id);
    let child = persons.get(h2_child.as_str()).copied().expect("H2 child");
    assert_eq!(child.generation, GenerationBand::Young);
    assert!(supporting.contains(child.id.as_str()));
    assert!(!actors.contains_key(child.id.as_str()));
    assert_eq!(derived.realm_id, "realm-01");
    assert_eq!(
        derived.succession_target_key,
        restored.succession_target_key
    );
    assert_eq!(derived.generation_distance, DERIVED_GENERATION_DISTANCE);
}

#[test]
fn each_realm_h2_lineage_and_effective_parents() {
    for seed in SEEDS {
        let world = generate_claim_propagation_world(seed).expect("propagation");
        for realm in &world.family_world.rights_world.rights.realms {
            let (h0, _h1, h2) = classify_realm_houses(&world, realm.realm_id.as_str());
            let restored = restored_claim_for_realm(&world, realm.realm_id.as_str());
            let derived = derived_for_realm(&world, realm.realm_id.as_str());
            assert_eq!(
                restored.claimant_person_id, h2.head_person_id,
                "seed={seed}"
            );
            let expected_child = h2.member_ids.get(5).expect("H2[5]");
            assert_eq!(&derived.claimant_person_id, expected_child, "seed={seed}");
            assert_eq!(
                derived.via_parent_person_id, h2.head_person_id,
                "seed={seed}"
            );
            let h0_current = h0.member_ids.get(3).expect("H0[3]");
            let parents = effective_parent_ids(
                &world.family_world.rights_world,
                &world.family_world.family,
                expected_child,
            )
            .expect("effective");
            assert_eq!(parents.len(), 2, "seed={seed}");
            let parent_set = parent_id_set(&parents);
            assert!(
                parent_set.contains(h2.head_person_id.as_str()),
                "seed={seed}"
            );
            assert!(parent_set.contains(h0_current.as_str()), "seed={seed}");
        }
    }
}

#[test]
fn direct_source_claimants_are_not_parents() {
    for seed in SEEDS {
        let world = generate_claim_propagation_world(seed).expect("propagation");
        for claim in &world.family_world.rights_world.rights.claims {
            if claim.basis != ClaimBasis::DirectDescent {
                continue;
            }
            for link in &world.family_world.family.parentages {
                assert!(
                    !link
                        .parent_person_ids
                        .iter()
                        .any(|id| id == &claim.claimant_person_id),
                    "seed={seed} direct claimant {} is a parent in {}",
                    claim.claimant_person_id,
                    link.id
                );
            }
        }
        let claims = claim_by_id(&world);
        for derived in &world.propagation.derived_claims {
            let source = claims
                .get(derived.source_claim_id.as_str())
                .copied()
                .expect("source");
            assert_ne!(source.basis, ClaimBasis::DirectDescent, "seed={seed}");
        }
    }
}

#[test]
fn spouse_only_does_not_receive_derived_claim() {
    for seed in SEEDS {
        let world = generate_claim_propagation_world(seed).expect("propagation");
        let derived_claimants: BTreeSet<&str> = world
            .propagation
            .derived_claims
            .iter()
            .map(|c| c.claimant_person_id.as_str())
            .collect();
        let mut spouse_only = 0usize;
        for realm in &world.family_world.rights_world.rights.realms {
            let restored = restored_claim_for_realm(&world, realm.realm_id.as_str());
            let marriage = marriage_containing(&world, restored.claimant_person_id.as_str());
            let spouse = marriage
                .spouse_person_ids
                .iter()
                .find(|id| *id != &restored.claimant_person_id)
                .expect("spouse");
            assert!(
                !derived_claimants.contains(spouse.as_str()),
                "seed={seed} spouse {spouse} received a derived claim"
            );
            spouse_only += 1;
        }
        assert_eq!(spouse_only, 6, "seed={seed}");
    }
}

#[test]
fn derived_source_is_traceable() {
    for seed in SEEDS {
        let world = generate_claim_propagation_world(seed).expect("propagation");
        let claims = claim_by_id(&world);
        for derived in &world.propagation.derived_claims {
            let source = claims
                .get(derived.source_claim_id.as_str())
                .copied()
                .expect("source exists");
            assert_eq!(
                source.claimant_person_id, derived.via_parent_person_id,
                "seed={seed}"
            );
            let parents = effective_parent_ids(
                &world.family_world.rights_world,
                &world.family_world.family,
                derived.claimant_person_id.as_str(),
            )
            .expect("effective");
            assert!(
                parents.iter().any(|id| id == &derived.via_parent_person_id),
                "seed={seed}"
            );
        }
    }
}

#[test]
fn derived_ids_follow_realm_order() {
    let world = generate_claim_propagation_world(1).expect("propagation");
    for (idx, derived) in world.propagation.derived_claims.iter().enumerate() {
        assert_eq!(derived.id, format!("derived-claim-{:02}", idx + 1));
        assert_eq!(derived.realm_id, format!("realm-{:02}", idx + 1));
    }
}

#[test]
fn arrays_explicitly_sorted() {
    let world = generate_claim_propagation_world(1).expect("propagation");
    let ids: Vec<_> = world
        .propagation
        .derived_claims
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[test]
fn malformed_propagation_fail_closed_no_panic() {
    let world = generate_claim_propagation_world(1).expect("propagation");
    let fw = &world.family_world;
    let (h0, _h1, h2) = classify_realm_houses(&world, "realm-01");

    let empty = InitialClaimPropagation {
        derived_claims: vec![],
    };
    let err = validate_initial_claim_propagation(fw, &empty).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut missing = world.propagation.clone();
    missing.derived_claims.pop();
    let err = validate_initial_claim_propagation(fw, &missing).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut dup_id = world.propagation.clone();
    if let (Some(first), Some(last)) = (
        dup_id.derived_claims.first().cloned(),
        dup_id.derived_claims.last_mut(),
    ) {
        last.id = first.id;
    }
    let err = validate_initial_claim_propagation(fw, &dup_id).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut unknown_source = world.propagation.clone();
    if let Some(first) = unknown_source.derived_claims.first_mut() {
        first.source_claim_id = "claim-does-not-exist".to_string();
    }
    let err = validate_initial_claim_propagation(fw, &unknown_source).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut unknown_claimant = world.propagation.clone();
    if let Some(first) = unknown_claimant.derived_claims.first_mut() {
        first.claimant_person_id = "person-does-not-exist".to_string();
    }
    let err = validate_initial_claim_propagation(fw, &unknown_claimant).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut unknown_parent = world.propagation.clone();
    if let Some(first) = unknown_parent.derived_claims.first_mut() {
        first.via_parent_person_id = "person-does-not-exist".to_string();
    }
    let err = validate_initial_claim_propagation(fw, &unknown_parent).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut via_mismatch = world.propagation.clone();
    if let Some(first) = via_mismatch.derived_claims.first_mut() {
        first.via_parent_person_id = h0.head_person_id.clone();
    }
    let err = validate_initial_claim_propagation(fw, &via_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut realm_mismatch = world.propagation.clone();
    if let Some(first) = realm_mismatch.derived_claims.first_mut() {
        first.realm_id = "realm-02".to_string();
    }
    let err = validate_initial_claim_propagation(fw, &realm_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut target_mismatch = world.propagation.clone();
    if let Some(first) = target_mismatch.derived_claims.first_mut() {
        first.succession_target_key = "succession:realm-02".to_string();
    }
    let err = validate_initial_claim_propagation(fw, &target_mismatch).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut distance_zero = world.propagation.clone();
    if let Some(first) = distance_zero.derived_claims.first_mut() {
        first.generation_distance = 0;
    }
    let err = validate_initial_claim_propagation(fw, &distance_zero).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut distance_two = world.propagation.clone();
    if let Some(first) = distance_two.derived_claims.first_mut() {
        first.generation_distance = 2;
    }
    let err = validate_initial_claim_propagation(fw, &distance_two).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut provenance_dup = world.propagation.clone();
    if let (Some(first), Some(second)) = (
        provenance_dup.derived_claims.first().cloned(),
        provenance_dup.derived_claims.get_mut(1),
    ) {
        second.source_claim_id = first.source_claim_id;
        second.via_parent_person_id = first.via_parent_person_id;
        second.claimant_person_id = first.claimant_person_id;
        second.claimant_house_id = first.claimant_house_id;
        second.realm_id = first.realm_id;
        second.succession_target_key = first.succession_target_key;
    }
    let err = validate_initial_claim_propagation(fw, &provenance_dup).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let direct = direct_claim_for_realm(&world, "realm-01");
    let mut not_restored = world.propagation.clone();
    if let Some(first) = not_restored.derived_claims.first_mut() {
        first.source_claim_id = direct.id.clone();
        first.via_parent_person_id = direct.claimant_person_id.clone();
    }
    let err = validate_initial_claim_propagation(fw, &not_restored).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut not_young = world.propagation.clone();
    if let Some(first) = not_young.derived_claims.first_mut() {
        first.claimant_person_id = h2.head_person_id.clone();
        first.claimant_house_id = h2.id.clone();
    }
    let err = validate_initial_claim_propagation(fw, &not_young).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut not_h2_expected = world.propagation.clone();
    if let Some(first) = not_h2_expected.derived_claims.first_mut() {
        first.claimant_person_id = h2.member_ids.get(6).expect("other H2 young").clone();
    }
    let err = validate_initial_claim_propagation(fw, &not_h2_expected).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut wrong_house = world.propagation.clone();
    if let Some(first) = wrong_house.derived_claims.first_mut() {
        first.claimant_house_id = h0.id.clone();
    }
    let err = validate_initial_claim_propagation(fw, &wrong_house).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut bad_family_schema = fw.clone();
    bad_family_schema.schema_version = 99;
    let err =
        validate_initial_claim_propagation(&bad_family_schema, &world.propagation).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let mut bad_family_seed = fw.clone();
    bad_family_seed.seed = fw.seed.wrapping_add(1);
    let err = validate_initial_claim_propagation(&bad_family_seed, &world.propagation).unwrap_err();
    assert!(
        matches!(err, CoreError::InvalidClaimPropagation(_)),
        "got {err:?}"
    );

    let ok = derive_initial_claim_propagation(fw).expect("derive");
    assert_eq!(ok.derived_claims.len(), DERIVED_CLAIM_COUNT);
}

#[test]
fn cli_claim_propagation_1_matches_generated_world() {
    let expected = generate_claim_propagation_world(1).expect("expected");
    common::assert_cli_json_eq(&["claim-propagation", "1"], &expected);
}

#[test]
fn cli_claim_propagation_check_1_and_2_print_exact_ok() {
    for seed in [1u64, 2] {
        common::assert_cli_exact(
            &["claim-propagation-check", &seed.to_string()],
            &expected_check_line(seed),
        );
    }
}

#[test]
fn existing_m0_to_m21_exact_regression() {
    common::assert_cli_exact_regression();
}
