// M2.1 초기 혼인·혈통망 — 가문 간 혼인·양친 연결 통합 테스트

use epoch_core::{
    ActiveRole, CONTEXT_WORLD_SCHEMA_VERSION, ClaimBasis, CoreError, DYNASTIC_WORLD_SCHEMA_VERSION,
    FAMILY_WORLD_SCHEMA_VERSION, FamilyWorld, GenerationBand, InitialFamilyNetwork, MARRIAGE_COUNT,
    PARENTAGE_COUNT, POLITICAL_WORLD_SCHEMA_VERSION, RIGHTS_WORLD_SCHEMA_VERSION,
    SAVE_SCHEMA_VERSION, WORLD_SCHEMA_VERSION, derive_initial_family, effective_parent_ids,
    generate_dynastic_world, generate_family_world, generate_rights_world, validate_initial_family,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

const SEEDS: [u64; 5] = [0, 1, 2, 42, u64::MAX];

fn run_epoch_lab(args: &[&str]) -> std::process::Output {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop(); // crates
    root.pop(); // workspace root
    Command::new("cargo")
        .current_dir(&root)
        .args(["run", "-q", "-p", "epoch-lab", "--"])
        .args(args)
        .output()
        .expect("cargo run -p epoch-lab")
}

fn person_by_id(
    world: &epoch_core::FamilyWorld,
) -> BTreeMap<&str, &epoch_core::population::Person> {
    world
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

fn house_by_id(world: &epoch_core::FamilyWorld) -> BTreeMap<&str, &epoch_core::population::House> {
    world
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
    world: &epoch_core::FamilyWorld,
) -> BTreeMap<&str, &epoch_core::HouseIdentity> {
    world
        .rights_world
        .context_world
        .context
        .house_identities
        .iter()
        .map(|h| (h.house_id.as_str(), h))
        .collect()
}

fn realm_identity_by_id(
    world: &epoch_core::FamilyWorld,
) -> BTreeMap<&str, &epoch_core::RealmIdentity> {
    world
        .rights_world
        .context_world
        .context
        .realm_identities
        .iter()
        .map(|r| (r.realm_id.as_str(), r))
        .collect()
}

fn person_identity_by_id(
    world: &epoch_core::FamilyWorld,
) -> BTreeMap<&str, &epoch_core::PersonIdentity> {
    world
        .rights_world
        .context_world
        .context
        .person_identities
        .iter()
        .map(|p| (p.person_id.as_str(), p))
        .collect()
}

fn houses_for_realm<'a>(
    world: &'a epoch_core::FamilyWorld,
    realm_id: &str,
) -> Vec<&'a epoch_core::population::House> {
    world
        .rights_world
        .context_world
        .political
        .dynastic
        .population
        .houses
        .iter()
        .filter(|h| h.realm_id == realm_id)
        .collect()
}

fn classify_realm_houses<'a>(
    world: &'a epoch_core::FamilyWorld,
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
        .rights_world
        .rights
        .realms
        .iter()
        .find(|r| r.realm_id == realm_id)
        .expect("realm rights");
    let incumbent = persons[rr.incumbent_person_id.as_str()];
    let h0 = houses[incumbent.house_id.as_str()];
    let ri = realm_identities[realm_id];
    let mut h1 = None;
    let mut h2 = None;
    for house in houses_for_realm(world, realm_id) {
        if house.id == h0.id {
            continue;
        }
        let hi = house_identities[house.id.as_str()];
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

#[test]
fn counts_shape_and_identity_patterns() {
    for seed in SEEDS {
        let fw = generate_family_world(seed).expect("family");
        assert_eq!(fw.schema_version, FAMILY_WORLD_SCHEMA_VERSION);
        assert_eq!(fw.seed, seed);
        assert_eq!(fw.family.marriages.len(), MARRIAGE_COUNT, "seed={seed}");
        assert_eq!(fw.family.parentages.len(), PARENTAGE_COUNT, "seed={seed}");

        let mut spouses = BTreeSet::new();
        let mut children = BTreeSet::new();
        let mut by_realm: BTreeMap<&str, usize> = BTreeMap::new();
        let mut same_house = 0usize;
        let mut interfaith = 0usize;
        let mut intercultural = 0usize;
        let identities = person_identity_by_id(&fw);
        for marriage in &fw.family.marriages {
            assert_eq!(marriage.spouse_person_ids.len(), 2, "seed={seed}");
            assert_eq!(marriage.house_ids.len(), 2, "seed={seed}");
            assert_eq!(marriage.realm_ids.len(), 1, "seed={seed}");
            assert_ne!(
                marriage.spouse_person_ids[0], marriage.spouse_person_ids[1],
                "seed={seed}"
            );
            assert_ne!(marriage.house_ids[0], marriage.house_ids[1], "seed={seed}");
            if marriage.house_ids[0] == marriage.house_ids[1] {
                same_house += 1;
            }
            for id in &marriage.spouse_person_ids {
                assert!(spouses.insert(id.as_str()), "seed={seed} spouse {id}");
            }
            *by_realm.entry(marriage.realm_ids[0].as_str()).or_insert(0) += 1;
            let left = identities[marriage.spouse_person_ids[0].as_str()];
            let right = identities[marriage.spouse_person_ids[1].as_str()];
            if left.culture_id == right.culture_id && left.religion_id != right.religion_id {
                interfaith += 1;
            }
            if left.culture_id != right.culture_id && left.religion_id == right.religion_id {
                intercultural += 1;
            }
        }
        for link in &fw.family.parentages {
            assert_eq!(link.parent_person_ids.len(), 2, "seed={seed}");
            assert!(
                children.insert(link.child_person_id.as_str()),
                "seed={seed} child {}",
                link.child_person_id
            );
        }
        assert_eq!(spouses.len(), 24, "seed={seed}");
        assert_eq!(children.len(), 12, "seed={seed}");
        assert_eq!(same_house, 0, "seed={seed}");
        assert_eq!(interfaith, 6, "seed={seed}");
        assert_eq!(intercultural, 6, "seed={seed}");
        assert_eq!(by_realm.len(), 6, "seed={seed}");
        for (realm_id, count) in &by_realm {
            assert_eq!(*count, 2, "seed={seed} realm={realm_id}");
        }

        let mut marriage_a = 0usize;
        let mut marriage_b = 0usize;
        for (idx, marriage) in fw.family.marriages.iter().enumerate() {
            if idx % 2 == 0 {
                marriage_a += 1;
            } else {
                marriage_b += 1;
            }
            assert!(marriage.id.starts_with("marriage-"), "seed={seed}");
        }
        assert_eq!(marriage_a, 6, "seed={seed}");
        assert_eq!(marriage_b, 6, "seed={seed}");
    }
}

#[test]
fn spouses_are_current_active_and_match_active_roster() {
    for seed in SEEDS {
        let fw = generate_family_world(seed).expect("family");
        let persons = person_by_id(&fw);
        let active: BTreeSet<&str> = fw
            .rights_world
            .context_world
            .political
            .roster
            .active_actors
            .iter()
            .map(|a| a.person_id.as_str())
            .collect();
        let supporting: BTreeSet<&str> = fw
            .rights_world
            .context_world
            .political
            .roster
            .supporting_person_ids
            .iter()
            .map(|s| s.as_str())
            .collect();
        let mut married = BTreeSet::new();
        for marriage in &fw.family.marriages {
            for id in &marriage.spouse_person_ids {
                let person = persons[id.as_str()];
                assert_eq!(person.generation, GenerationBand::Current, "seed={seed}");
                assert!(active.contains(id.as_str()), "seed={seed} {id}");
                assert!(!supporting.contains(id.as_str()), "seed={seed} {id}");
                assert!(married.insert(id.as_str()), "seed={seed} {id}");
            }
        }
        assert_eq!(married, active, "seed={seed}");
        assert_eq!(married.len(), 24, "seed={seed}");
        assert_eq!(supporting.intersection(&married).count(), 0, "seed={seed}");
    }
}

#[test]
fn parentage_preserves_known_parent_and_matches_spouses() {
    for seed in SEEDS {
        let fw = generate_family_world(seed).expect("family");
        let persons = person_by_id(&fw);
        let marriage_by_id: BTreeMap<_, _> = fw
            .family
            .marriages
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect();
        for link in &fw.family.parentages {
            let child = persons[link.child_person_id.as_str()];
            assert_eq!(child.generation, GenerationBand::Young, "seed={seed}");
            let marriage = marriage_by_id[link.marriage_id.as_str()];
            assert_eq!(
                link.parent_person_ids, marriage.spouse_person_ids,
                "seed={seed}"
            );
            let known: BTreeSet<&str> = child.known_parent_ids.iter().map(String::as_str).collect();
            let parents: BTreeSet<&str> =
                link.parent_person_ids.iter().map(String::as_str).collect();
            assert!(
                known.is_subset(&parents),
                "seed={seed} known parent dropped for {}",
                child.id
            );
            assert_eq!(known.len(), 1, "seed={seed}");
            assert_eq!(known.intersection(&parents).count(), 1, "seed={seed}");
            let left = persons[link.parent_person_ids[0].as_str()];
            let right = persons[link.parent_person_ids[1].as_str()];
            assert_eq!(left.generation, GenerationBand::Current);
            assert_eq!(right.generation, GenerationBand::Current);
            assert_ne!(left.house_id, right.house_id);
            assert!(child.house_id == left.house_id || child.house_id == right.house_id);
        }
    }
}

#[test]
fn direct_claimant_and_h2_young_effective_parents() {
    for seed in SEEDS {
        let fw = generate_family_world(seed).expect("family");
        let persons = person_by_id(&fw);
        let parentage_by_child: BTreeMap<_, _> = fw
            .family
            .parentages
            .iter()
            .map(|p| (p.child_person_id.as_str(), p))
            .collect();
        for realm in &fw.rights_world.rights.realms {
            let (h0, h1, h2) = classify_realm_houses(&fw, &realm.realm_id);
            let direct = fw
                .rights_world
                .rights
                .claims
                .iter()
                .find(|c| c.realm_id == realm.realm_id && c.basis == ClaimBasis::DirectDescent)
                .expect("direct");
            let expected_direct = h0.member_ids.get(5).expect("H0[5]");
            assert_eq!(&direct.claimant_person_id, expected_direct);
            let a_link = parentage_by_child[direct.claimant_person_id.as_str()];
            assert_eq!(a_link.child_person_id, *expected_direct);
            let a_parents: BTreeSet<&str> = a_link
                .parent_person_ids
                .iter()
                .map(String::as_str)
                .collect();
            assert_eq!(a_parents.len(), 2);
            assert!(a_parents.contains(h0.head_person_id.as_str()));
            assert!(a_parents.contains(h1.head_person_id.as_str()));
            let effective = effective_parent_ids(&fw.rights_world, &fw.family, expected_direct)
                .expect("effective A");
            assert_eq!(effective, a_link.parent_person_ids);

            let h2_child = h2.member_ids.get(5).expect("H2[5]");
            let b_link = parentage_by_child[h2_child.as_str()];
            let b_parents: BTreeSet<&str> = b_link
                .parent_person_ids
                .iter()
                .map(String::as_str)
                .collect();
            let h0_current = h0.member_ids.get(3).expect("H0[3]");
            assert!(b_parents.contains(h2.head_person_id.as_str()));
            assert!(b_parents.contains(h0_current.as_str()));
            let child = persons[h2_child.as_str()];
            assert_eq!(child.generation, GenerationBand::Young);
            let effective_b =
                effective_parent_ids(&fw.rights_world, &fw.family, h2_child).expect("effective B");
            assert_eq!(effective_b, b_link.parent_person_ids);
        }
    }
}

#[test]
fn seed1_realm01_concrete_trace() {
    let fw = generate_family_world(1).expect("family");
    let persons = person_by_id(&fw);
    let identities = person_identity_by_id(&fw);
    let (h0, h1, h2) = classify_realm_houses(&fw, "realm-01");
    let rr = fw
        .rights_world
        .rights
        .realms
        .iter()
        .find(|r| r.realm_id == "realm-01")
        .expect("realm-01");
    assert_eq!(rr.incumbent_person_id, h0.head_person_id);

    let marriage_a = fw
        .family
        .marriages
        .iter()
        .find(|m| m.id == "marriage-01")
        .expect("marriage-01");
    let marriage_b = fw
        .family
        .marriages
        .iter()
        .find(|m| m.id == "marriage-02")
        .expect("marriage-02");
    let a_spouses: BTreeSet<&str> = marriage_a
        .spouse_person_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert!(a_spouses.contains(rr.incumbent_person_id.as_str()));
    assert!(a_spouses.contains(h1.head_person_id.as_str()));
    for id in &marriage_a.spouse_person_ids {
        let person = persons[id.as_str()];
        assert_eq!(person.generation, GenerationBand::Current);
        assert!(
            fw.rights_world
                .context_world
                .political
                .roster
                .active_actors
                .iter()
                .any(|a| a.person_id == *id)
        );
    }
    let a_left = identities[marriage_a.spouse_person_ids[0].as_str()];
    let a_right = identities[marriage_a.spouse_person_ids[1].as_str()];
    assert_eq!(a_left.culture_id, a_right.culture_id);
    assert_ne!(a_left.religion_id, a_right.religion_id);

    let direct = fw
        .rights_world
        .rights
        .claims
        .iter()
        .find(|c| c.realm_id == "realm-01" && c.basis == ClaimBasis::DirectDescent)
        .expect("direct");
    let h0_child = h0.member_ids.get(5).expect("H0[5]");
    assert_eq!(&direct.claimant_person_id, h0_child);
    let parentage_a = fw
        .family
        .parentages
        .iter()
        .find(|p| p.id == "parentage-01")
        .expect("parentage-01");
    assert_eq!(&parentage_a.child_person_id, h0_child);
    let child_a = persons[h0_child.as_str()];
    assert_eq!(child_a.generation, GenerationBand::Young);
    let parents_a =
        effective_parent_ids(&fw.rights_world, &fw.family, h0_child).expect("effective A");
    assert_eq!(parents_a.len(), 2);
    assert!(parents_a.contains(&rr.incumbent_person_id));
    assert!(parents_a.contains(&h1.head_person_id));

    let b_spouses: BTreeSet<&str> = marriage_b
        .spouse_person_ids
        .iter()
        .map(String::as_str)
        .collect();
    let h0_current = h0.member_ids.get(3).expect("H0[3]");
    let restored = fw
        .rights_world
        .rights
        .claims
        .iter()
        .find(|c| c.realm_id == "realm-01" && c.basis == ClaimBasis::RestoredLineRecord)
        .expect("restored");
    assert_eq!(restored.claimant_person_id, h2.head_person_id);
    assert!(b_spouses.contains(h0_current.as_str()));
    assert!(b_spouses.contains(h2.head_person_id.as_str()));
    for id in &marriage_b.spouse_person_ids {
        let person = persons[id.as_str()];
        assert_eq!(person.generation, GenerationBand::Current);
        assert!(
            fw.rights_world
                .context_world
                .political
                .roster
                .active_actors
                .iter()
                .any(|a| a.person_id == *id)
        );
    }
    let b_left = identities[marriage_b.spouse_person_ids[0].as_str()];
    let b_right = identities[marriage_b.spouse_person_ids[1].as_str()];
    assert_ne!(b_left.culture_id, b_right.culture_id);
    assert_eq!(b_left.religion_id, b_right.religion_id);

    let h2_child = h2.member_ids.get(5).expect("H2[5]");
    let parentage_b = fw
        .family
        .parentages
        .iter()
        .find(|p| p.id == "parentage-02")
        .expect("parentage-02");
    assert_eq!(&parentage_b.child_person_id, h2_child);
    let child_b = persons[h2_child.as_str()];
    assert_eq!(child_b.generation, GenerationBand::Young);
    let parents_b =
        effective_parent_ids(&fw.rights_world, &fw.family, h2_child).expect("effective B");
    assert_eq!(parents_b.len(), 2);
    assert!(parents_b.contains(&h2.head_person_id));
    assert!(parents_b.contains(h0_current));
}

#[test]
fn effective_parent_ids_helper() {
    let fw = generate_family_world(1).expect("family");
    let persons = person_by_id(&fw);
    let first_link = fw.family.parentages.first().expect("parentage");
    let from_helper = effective_parent_ids(
        &fw.rights_world,
        &fw.family,
        first_link.child_person_id.as_str(),
    )
    .expect("child");
    assert_eq!(from_helper, first_link.parent_person_ids);

    let elder = persons
        .values()
        .find(|p| p.generation == GenerationBand::Elder)
        .expect("elder");
    let known =
        effective_parent_ids(&fw.rights_world, &fw.family, elder.id.as_str()).expect("elder");
    assert_eq!(known, elder.known_parent_ids);

    let err = effective_parent_ids(&fw.rights_world, &fw.family, "person-does-not-exist")
        .expect_err("unknown");
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");
}

#[test]
fn same_seed_equality_and_bytes() {
    let a = generate_family_world(1).expect("a");
    let b = generate_family_world(1).expect("b");
    assert_eq!(a, b);
    assert_eq!(
        a.to_compact_json_bytes().unwrap(),
        b.to_compact_json_bytes().unwrap()
    );
    let c = generate_family_world(2).expect("c");
    assert_ne!(
        a.to_compact_json_bytes().unwrap(),
        c.to_compact_json_bytes().unwrap()
    );
    assert_eq!(c.family.marriages.len(), 12);
    assert_eq!(c.family.parentages.len(), 12);
}

#[test]
fn lower_layer_bytes_and_rng_unchanged() {
    for seed in SEEDS {
        let rights = generate_rights_world(seed).expect("rights");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        let fw = generate_family_world(seed).expect("family");
        assert_eq!(fw.rights_world, rights, "seed={seed}");
        assert_eq!(
            fw.rights_world.to_compact_json_bytes().unwrap(),
            rights.to_compact_json_bytes().unwrap(),
            "seed={seed}"
        );
        assert_eq!(
            fw.rights_world.context_world, rights.context_world,
            "seed={seed}"
        );
        assert_eq!(
            fw.rights_world.context_world.political, rights.context_world.political,
            "seed={seed}"
        );
        assert_eq!(
            fw.rights_world.context_world.political.dynastic.population, dynastic.population,
            "seed={seed}"
        );
        assert_eq!(
            fw.rights_world
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
            fw.rights_world
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
            fw.rights_world.rights.claims.len(),
            12,
            "seed={seed} claims must stay 12"
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
}

#[test]
fn malformed_family_fail_closed_no_panic() {
    let fw = generate_family_world(1).expect("family");
    let rw = &fw.rights_world;

    let empty = InitialFamilyNetwork {
        marriages: vec![],
        parentages: vec![],
    };
    let err = validate_initial_family(rw, &empty).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut missing_marriage = fw.family.clone();
    missing_marriage.marriages.pop();
    let err = validate_initial_family(rw, &missing_marriage).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut dup_marriage = fw.family.clone();
    if let Some(first) = dup_marriage.marriages.first().cloned()
        && let Some(last) = dup_marriage.marriages.last_mut()
    {
        last.id = first.id;
    }
    let err = validate_initial_family(rw, &dup_marriage).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut missing_parentage = fw.family.clone();
    missing_parentage.parentages.pop();
    let err = validate_initial_family(rw, &missing_parentage).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut dup_parentage = fw.family.clone();
    if let Some(first) = dup_parentage.parentages.first().cloned()
        && let Some(last) = dup_parentage.parentages.last_mut()
    {
        last.id = first.id;
    }
    let err = validate_initial_family(rw, &dup_parentage).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut spouse_count = fw.family.clone();
    if let Some(first) = spouse_count.marriages.first_mut() {
        first.spouse_person_ids.pop();
    }
    let err = validate_initial_family(rw, &spouse_count).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut parent_count = fw.family.clone();
    if let Some(first) = parent_count.parentages.first_mut() {
        first.parent_person_ids.pop();
    }
    let err = validate_initial_family(rw, &parent_count).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut unknown_person = fw.family.clone();
    if let Some(first) = unknown_person.marriages.first_mut()
        && let Some(id) = first.spouse_person_ids.first_mut()
    {
        *id = "person-does-not-exist".to_string();
    }
    let err = validate_initial_family(rw, &unknown_person).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut unknown_house = fw.family.clone();
    if let Some(first) = unknown_house.marriages.first_mut()
        && let Some(id) = first.house_ids.first_mut()
    {
        *id = "house-does-not-exist".to_string();
    }
    let err = validate_initial_family(rw, &unknown_house).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut realm_mismatch = fw.family.clone();
    if let Some(first) = realm_mismatch.marriages.first_mut()
        && let Some(id) = first.realm_ids.first_mut()
    {
        *id = "realm-02".to_string();
    }
    let err = validate_initial_family(rw, &realm_mismatch).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut same_house = fw.family.clone();
    if let Some(first) = same_house.marriages.first_mut()
        && let Some(house) = first.house_ids.first().cloned()
    {
        first.house_ids = vec![house.clone(), house];
    }
    let err = validate_initial_family(rw, &same_house).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut spouse_dup = fw.family.clone();
    if let (Some(first), Some(second)) = (
        spouse_dup.marriages.first().cloned(),
        spouse_dup.marriages.get_mut(1),
    ) && let (Some(src), Some(dst)) = (
        first.spouse_person_ids.first(),
        second.spouse_person_ids.first_mut(),
    ) {
        *dst = src.clone();
    }
    let err = validate_initial_family(rw, &spouse_dup).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut child_dup = fw.family.clone();
    if let (Some(first), Some(second)) = (
        child_dup.parentages.first().cloned(),
        child_dup.parentages.get_mut(1),
    ) {
        second.child_person_id = first.child_person_id;
    }
    let err = validate_initial_family(rw, &child_dup).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let (h0, _h1, h2) = classify_realm_houses(&fw, "realm-01");
    let mut not_current = fw.family.clone();
    if let Some(first) = not_current.marriages.first_mut()
        && let Some(id) = first.spouse_person_ids.first_mut()
    {
        *id = h0.member_ids.get(5).expect("young").clone();
        first.spouse_person_ids.sort();
    }
    let err = validate_initial_family(rw, &not_current).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut not_young = fw.family.clone();
    if let Some(first) = not_young.parentages.first_mut() {
        first.child_person_id = h0.head_person_id.clone();
    }
    let err = validate_initial_family(rw, &not_young).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut parents_mismatch = fw.family.clone();
    if let Some(first) = parents_mismatch.parentages.first_mut() {
        first.parent_person_ids = vec!["person-001".to_string(), "person-002".to_string()];
        first.parent_person_ids.sort();
    }
    let err = validate_initial_family(rw, &parents_mismatch).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut known_parent_missing = fw.family.clone();
    if let Some(first) = known_parent_missing.parentages.first_mut()
        && let Some(second) = first.parent_person_ids.get_mut(0)
    {
        *second = h2.head_person_id.clone();
        first.parent_person_ids.sort();
        first.parent_person_ids.dedup();
        if first.parent_person_ids.len() == 1 {
            first
                .parent_person_ids
                .push(h0.member_ids.get(3).expect("H0[3]").clone());
            first.parent_person_ids.sort();
        }
    }
    let err = validate_initial_family(rw, &known_parent_missing).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut not_direct_child = fw.family.clone();
    if let Some(first) = not_direct_child.parentages.first_mut() {
        first.child_person_id = h0.member_ids.get(6).expect("other young").clone();
    }
    let err = validate_initial_family(rw, &not_direct_child).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut not_h2_child = fw.family.clone();
    if let Some(second) = not_h2_child.parentages.get_mut(1) {
        second.child_person_id = h2.member_ids.get(6).expect("other H2 young").clone();
    }
    let err = validate_initial_family(rw, &not_h2_child).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let mut identity_swap = fw.family.clone();
    if let Some(first) = identity_swap.marriages.first_mut() {
        first.spouse_person_ids = vec![h0.head_person_id.clone(), h2.head_person_id.clone()];
        first.spouse_person_ids.sort();
        first.house_ids = vec![h0.id.clone(), h2.id.clone()];
        first.house_ids.sort();
    }
    if let Some(first) = identity_swap.parentages.first_mut() {
        first.parent_person_ids = identity_swap
            .marriages
            .first()
            .expect("marriage")
            .spouse_person_ids
            .clone();
    }
    let err = validate_initial_family(rw, &identity_swap).unwrap_err();
    assert!(matches!(err, CoreError::InvalidFamily(_)), "got {err:?}");

    let ok = derive_initial_family(rw).expect("derive");
    assert_eq!(ok.marriages.len(), 12);
    assert_eq!(ok.parentages.len(), 12);
}

#[test]
fn arrays_explicitly_sorted() {
    let fw = generate_family_world(1).expect("family");
    let marriage_ids: Vec<_> = fw.family.marriages.iter().map(|m| m.id.as_str()).collect();
    let mut sorted = marriage_ids.clone();
    sorted.sort();
    assert_eq!(marriage_ids, sorted);
    let parentage_ids: Vec<_> = fw.family.parentages.iter().map(|p| p.id.as_str()).collect();
    let mut sorted_p = parentage_ids.clone();
    sorted_p.sort();
    assert_eq!(parentage_ids, sorted_p);
    for marriage in &fw.family.marriages {
        let mut spouses = marriage.spouse_person_ids.clone();
        spouses.sort();
        assert_eq!(marriage.spouse_person_ids, spouses);
        let mut houses = marriage.house_ids.clone();
        houses.sort();
        assert_eq!(marriage.house_ids, houses);
        let mut realms = marriage.realm_ids.clone();
        realms.sort();
        assert_eq!(marriage.realm_ids, realms);
    }
    for link in &fw.family.parentages {
        let mut parents = link.parent_person_ids.clone();
        parents.sort();
        assert_eq!(link.parent_person_ids, parents);
    }
}

#[test]
fn cli_family_1_succeeds() {
    let output = run_epoch_lab(&["family", "1"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("family stdout utf-8");
    let world: FamilyWorld =
        serde_json::from_str(stdout.trim()).expect("family stdout must be FamilyWorld JSON");
    assert_eq!(world.schema_version, FAMILY_WORLD_SCHEMA_VERSION);
    assert_eq!(world.seed, 1);
    assert_eq!(world.family.marriages.len(), MARRIAGE_COUNT);
    assert_eq!(world.family.parentages.len(), PARENTAGE_COUNT);
    assert_eq!(world.family.marriages[0].id, "marriage-01");
    assert_eq!(world.family.parentages[0].id, "parentage-01");
    assert_eq!(world, generate_family_world(1).expect("expected family"));
}

#[test]
fn cli_family_check_1_and_2_print_family_ok() {
    for seed in [1u64, 2] {
        let output = run_epoch_lab(&["family-check", &seed.to_string()]);
        assert!(
            output.status.success(),
            "seed={seed} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected_bytes = generate_family_world(seed)
            .expect("family")
            .to_compact_json_bytes()
            .expect("family bytes")
            .len();
        let expected = format!(
            "FAMILY_OK seed={seed} marriages=12 parentages=12 interfaith=6 intercultural=6 dual_parent_children=12 bytes={expected_bytes}"
        );
        assert_eq!(stdout.trim(), expected, "seed={seed}");
    }
}

#[test]
fn existing_m1_and_m0_exact_regression() {
    let checks = [
        (
            ["rights-check", "1"],
            "RIGHTS_OK seed=1 realms=6 claims=12 direct=6 restored=6 strong=6 contested=6 evidence=6 bytes=66222",
        ),
        (
            ["rights-check", "2"],
            "RIGHTS_OK seed=2 realms=6 claims=12 direct=6 restored=6 strong=6 contested=6 evidence=6 bytes=66221",
        ),
        (
            ["context-check", "1"],
            "CONTEXT_OK seed=1 cultures=3 religions=2 realm_profiles=6 house_profiles=18 person_profiles=144 relations=24 promises=12 information=18 bytes=61898",
        ),
        (
            ["context-check", "2"],
            "CONTEXT_OK seed=2 cultures=3 religions=2 realm_profiles=6 house_profiles=18 person_profiles=144 relations=24 promises=12 information=18 bytes=61897",
        ),
        (
            ["actors-check", "1"],
            "ACTORS_OK seed=1 active=24 supporting=120 rulers=6 house_heads=12 ruling_house_current=6 realms=6 bytes=39466",
        ),
        (
            ["actors-check", "2"],
            "ACTORS_OK seed=2 active=24 supporting=120 rulers=6 house_heads=12 ruling_house_current=6 realms=6 bytes=39465",
        ),
        (
            ["population-check", "1"],
            "POPULATION_OK seed=1 houses=18 persons=144 elder=36 current=54 young=54 rulers=6 bytes=34960",
        ),
        (
            ["population-check", "2"],
            "POPULATION_OK seed=2 houses=18 persons=144 elder=36 current=54 young=54 rulers=6 bytes=34959",
        ),
        (
            ["world-check", "1"],
            "WORLD_OK seed=1 realms=6 territories=36 rulers=6 template=vertical bytes=6234",
        ),
        (
            ["world-check", "2"],
            "WORLD_OK seed=2 realms=6 territories=36 rulers=6 template=blocks_2x3 bytes=6233",
        ),
        (
            ["replay-check", "1"],
            "DETERMINISM_OK seed=1 events=5 bytes=7353",
        ),
        (
            ["replay-check", "2"],
            "DETERMINISM_OK seed=2 events=5 bytes=7392",
        ),
        (
            ["save-check", "1"],
            "SAVE_LOAD_OK seed=1 checkpoint_bytes=2167 events=5",
        ),
        (
            ["save-check", "2"],
            "SAVE_LOAD_OK seed=2 checkpoint_bytes=2167 events=5",
        ),
    ];
    for (args, expected) in checks {
        let output = run_epoch_lab(&args);
        assert!(
            output.status.success(),
            "{} stderr: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            expected,
            "{}",
            args[0]
        );
    }
}

#[test]
fn active_roles_of_married_spouses() {
    let fw = generate_family_world(1).expect("family");
    let actors: BTreeMap<_, _> = fw
        .rights_world
        .context_world
        .political
        .roster
        .active_actors
        .iter()
        .map(|a| (a.person_id.as_str(), a))
        .collect();
    let (h0, h1, h2) = classify_realm_houses(&fw, "realm-01");
    assert_eq!(
        actors[h0.head_person_id.as_str()].primary_role,
        ActiveRole::Ruler
    );
    assert_eq!(
        actors[h1.head_person_id.as_str()].primary_role,
        ActiveRole::HouseHead
    );
    assert_eq!(
        actors[h2.head_person_id.as_str()].primary_role,
        ActiveRole::HouseHead
    );
    assert_eq!(
        actors[h0.member_ids.get(3).expect("H0[3]").as_str()].primary_role,
        ActiveRole::RulingHouseCurrent
    );
}
