// M1.2 인구·가계 골격 CLI·결정론 통합 테스트

use epoch_core::{
    GenerationBand, HOUSE_COUNT, PERSON_COUNT, generate_dynastic_world, generate_world,
    validate_population, validate_world,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

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

#[test]
fn exact_counts_and_generations() {
    for seed in [0u64, 1, 2, 42, u64::MAX] {
        let d = generate_dynastic_world(seed).expect("dynastic");
        assert_eq!(d.population.houses.len(), HOUSE_COUNT);
        assert_eq!(d.population.persons.len(), PERSON_COUNT);
        assert_eq!(d.population.ruler_links.len(), 6);

        let mut elder = 0usize;
        let mut current = 0usize;
        let mut young = 0usize;
        for p in &d.population.persons {
            match p.generation {
                GenerationBand::Elder => elder += 1,
                GenerationBand::Current => current += 1,
                GenerationBand::Young => young += 1,
            }
        }
        assert_eq!(elder, 36);
        assert_eq!(current, 54);
        assert_eq!(young, 54);
    }
}

#[test]
fn realm_distribution_three_houses_twenty_four_persons() {
    let d = generate_dynastic_world(1).expect("dynastic");
    for realm in &d.world.realms {
        let houses: Vec<_> = d
            .population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        assert_eq!(houses.len(), 3, "realm {}", realm.id);
        let persons: Vec<_> = d
            .population
            .persons
            .iter()
            .filter(|p| p.realm_id == realm.id)
            .collect();
        assert_eq!(persons.len(), 24, "realm {}", realm.id);
    }
}

#[test]
fn house_generation_composition_and_head_current() {
    let d = generate_dynastic_world(42).expect("dynastic");
    let by_id: BTreeMap<_, _> = d
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    for house in &d.population.houses {
        assert_eq!(house.member_ids.len(), 8);
        let mut e = 0;
        let mut c = 0;
        let mut y = 0;
        for mid in &house.member_ids {
            match by_id[mid.as_str()].generation {
                GenerationBand::Elder => e += 1,
                GenerationBand::Current => c += 1,
                GenerationBand::Young => y += 1,
            }
        }
        assert_eq!((e, c, y), (2, 3, 3), "house {}", house.id);
        let head = by_id[house.head_person_id.as_str()];
        assert_eq!(head.generation, GenerationBand::Current);
        assert_eq!(head.home_territory_id, house.seat_territory_id);
        assert_eq!(head.realm_id, house.realm_id);
    }
}

#[test]
fn seats_unique_in_realm_and_first_is_capital() {
    let d = generate_dynastic_world(1).expect("dynastic");
    for realm in &d.world.realms {
        let mut houses: Vec<_> = d
            .population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(houses[0].seat_territory_id, realm.capital_territory_id);
        let mut seats = BTreeSet::new();
        for h in &houses {
            assert!(
                realm.territory_ids.contains(&h.seat_territory_id),
                "seat outside realm"
            );
            assert!(seats.insert(h.seat_territory_id.as_str()));
        }
    }
}

#[test]
fn parent_graph_same_house_previous_generation_no_cycle() {
    let d = generate_dynastic_world(2).expect("dynastic");
    let by_id: BTreeMap<_, _> = d
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    for p in &d.population.persons {
        let mut seen = BTreeSet::new();
        for parent_id in &p.known_parent_ids {
            assert_ne!(parent_id, &p.id, "self-parent");
            assert!(seen.insert(parent_id.as_str()), "dup parent");
            let parent = by_id[parent_id.as_str()];
            assert_eq!(parent.house_id, p.house_id);
            match p.generation {
                GenerationBand::Elder => panic!("elder has parent"),
                GenerationBand::Current => assert_eq!(parent.generation, GenerationBand::Elder),
                GenerationBand::Young => assert_eq!(parent.generation, GenerationBand::Current),
            }
        }
        match p.generation {
            GenerationBand::Elder => assert!(p.known_parent_ids.is_empty()),
            GenerationBand::Current | GenerationBand::Young => {
                assert!(!p.known_parent_ids.is_empty());
            }
        }
    }
}

#[test]
fn ruler_links_match_ruling_house_heads() {
    let d = generate_dynastic_world(1).expect("dynastic");
    assert_eq!(d.population.ruler_links.len(), 6);
    let person_by_id: BTreeMap<_, _> = d
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let mut linked_persons = BTreeSet::new();
    for ruler in &d.world.rulers {
        let link = d
            .population
            .ruler_links
            .iter()
            .find(|l| l.ruler_id == ruler.id)
            .expect("link");
        assert!(linked_persons.insert(link.person_id.as_str()));
        let person = person_by_id[link.person_id.as_str()];
        assert_eq!(person.name, ruler.name);
        assert_eq!(person.realm_id, ruler.realm_id);
        assert_eq!(person.generation, GenerationBand::Current);
        let realm = d
            .world
            .realms
            .iter()
            .find(|r| r.id == ruler.realm_id)
            .expect("realm");
        assert_eq!(person.home_territory_id, realm.capital_territory_id);
        let mut houses: Vec<_> = d
            .population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(houses[0].head_person_id, person.id);
        assert_eq!(houses[0].seat_territory_id, realm.capital_territory_id);
    }
}

#[test]
fn same_seed_equality_and_seed_1_2_differ() {
    let a = generate_dynastic_world(1).expect("a");
    let b = generate_dynastic_world(1).expect("b");
    assert_eq!(a, b);
    let ba = a.to_compact_json_bytes().unwrap();
    let bb = b.to_compact_json_bytes().unwrap();
    assert_eq!(ba, bb);

    let c = generate_dynastic_world(2).expect("c");
    assert_ne!(a, c);
    assert_ne!(ba, c.to_compact_json_bytes().unwrap());
}

#[test]
fn population_does_not_change_world_skeleton_bytes() {
    for seed in [1u64, 2] {
        let world = generate_world(seed).expect("world");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        assert_eq!(world, dynastic.world);
        assert_eq!(
            world.to_compact_json_bytes().unwrap(),
            dynastic.world.to_compact_json_bytes().unwrap()
        );
        validate_world(&dynastic.world).expect("world ok");
        validate_population(&dynastic.world, &dynastic.population).expect("pop ok");
    }
}

#[test]
fn person_ids_contiguous_per_house() {
    let d = generate_dynastic_world(0).expect("dynastic");
    for (i, house) in d.population.houses.iter().enumerate() {
        let base = i * 8;
        for (m, mid) in house.member_ids.iter().enumerate() {
            let expected = format!("person-{:03}", base + m + 1);
            assert_eq!(mid, &expected);
        }
    }
}

#[test]
fn full_invariants_on_representative_seeds() {
    for seed in [0u64, 1, 2, 42, u64::MAX] {
        let d = generate_dynastic_world(seed).expect("dynastic");
        validate_population(&d.world, &d.population).expect("invariants");
    }
}

#[test]
fn cli_population_1_succeeds() {
    let output = run_epoch_lab(&["population", "1"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"schema_version\""), "stdout: {stdout}");
    assert!(stdout.contains("\"population\""), "stdout: {stdout}");
    assert!(stdout.contains("\"houses\""), "stdout: {stdout}");
    assert!(stdout.contains("\"persons\""), "stdout: {stdout}");
    assert!(stdout.contains("\"ruler_links\""), "stdout: {stdout}");
}

#[test]
fn cli_population_check_1_prints_population_ok() {
    let output = run_epoch_lab(&["population-check", "1"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("POPULATION_OK"), "stdout: {stdout}");
    assert!(stdout.contains("seed=1"), "stdout: {stdout}");
    assert!(stdout.contains("houses=18"), "stdout: {stdout}");
    assert!(stdout.contains("persons=144"), "stdout: {stdout}");
    assert!(stdout.contains("elder=36"), "stdout: {stdout}");
    assert!(stdout.contains("current=54"), "stdout: {stdout}");
    assert!(stdout.contains("young=54"), "stdout: {stdout}");
    assert!(stdout.contains("rulers=6"), "stdout: {stdout}");
}

#[test]
fn cli_population_check_2_prints_population_ok() {
    let output = run_epoch_lab(&["population-check", "2"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("POPULATION_OK"), "stdout: {stdout}");
    assert!(stdout.contains("seed=2"), "stdout: {stdout}");
}
