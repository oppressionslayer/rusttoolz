//! The city, the one tick law and first contact, held to the files the site and the engine share.
use rusttoolz::{city, contact, motion};

fn read(name: &str) -> Option<serde_json::Value> {
    std::fs::read_to_string(format!("tests/data/{name}")).ok().map(|t| serde_json::from_str(&t).unwrap())
}

#[test]
fn the_city_matches_the_engine_district() {
    let Some(r) = read("district-45.rust.json") else { return };
    let d = city::district(45, &[]).unwrap();
    for (k, l) in d.lots.iter().enumerate() {
        let rl = &r["lots"][k];
        assert_eq!(l.fields.floors as u64, rl["floors"].as_u64().unwrap(), "lot {k}");
        assert_eq!(l.n.to_string(), rl["N"].as_str().unwrap(), "lot {k}");
        assert_eq!(l.source.1 as u64, rl["sourcePrime"].as_u64().unwrap());
        assert_eq!(l.geometry.bounds.y1, rl["bounds"][5].as_f64().unwrap());
        assert_eq!(city::open_card(l.n), Some((45, l.cell, l.fields)));
    }
    assert_eq!(d.streets.width, r["streets"]["width"].as_f64().unwrap());
    assert_eq!(city::reference_json(45), std::fs::read_to_string("tests/data/district-45.rust.json").unwrap().trim());
}

#[test]
fn the_one_tick_law_matches_the_studio_and_its_closed_form() {
    let Some(r) = read("driving-v1.javascript.json") else { return };
    for case in r["cases"].as_array().unwrap() {
        let h = &case["handling"];
        if h["grip"].as_f64().unwrap() < 1.0 {
            continue;
        }
        let (speed, accel, wet) = (h["speed"].as_f64().unwrap(), h["accel"].as_f64().unwrap(), h["wet"].as_f64().unwrap());
        let mut v = case["initial"]["speed"].as_f64().unwrap();
        let mut tick = 0i64;
        let cps: Vec<(i64, f64)> = case["checkpoints"].as_array().unwrap().iter().map(|c| (c["tick"].as_i64().unwrap(), c["speed"].as_f64().unwrap())).collect();
        for run in case["runs"].as_array().unwrap() {
            let (t, _, brake) = motion::decode_input(run["code"].as_i64().unwrap());
            for _ in 0..run["ticks"].as_i64().unwrap() {
                let upper = speed * if t < 0.0 { 0.45 } else { 1.0 };
                v = motion::longitudinal_step(v, t, accel, motion::drag_of(brake, wet), motion::DT, 0.45 * speed, upper);
                tick += 1;
                if let Some((_, want)) = cps.iter().find(|c| c.0 == tick) {
                    assert!((v - want).abs() < 1e-9, "{} tick {tick}: {v} vs {want}", case["name"]);
                }
            }
        }
    }
    assert!((motion::settles_at(1.0, 12.0, 0.7, motion::DT) - 17.042857142857).abs() < 1e-9);
    assert_eq!(motion::cap_tick(20.0, 16.0, 0.7, motion::DT, 10_000), Some(363));
    let mut v = 0.0;
    for _ in 0..120 {
        v = motion::longitudinal_step(v, 1.0, 12.0, 0.7, motion::DT, 13.5, 30.0);
    }
    assert!((motion::closed_form(0.0, 1.0, 12.0, 0.7, 120, motion::DT) - v).abs() < 1e-9);
    assert!((motion::distance(0.0, 1.0, 12.0, 0.7, 120, motion::DT) - 4.8331361692129).abs() < 1e-9);
}

#[test]
fn first_contact_matches_the_studio_scenes() {
    let Some(r) = read("contact-v1.reference.json") else { return };
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 56);
    for c in cases {
        let m = &c["motion"];
        let mo = contact::Motion { x: m["x"].as_f64().unwrap(), z: m["z"].as_f64().unwrap(), dx: m["dx"].as_f64().unwrap(), dz: m["dz"].as_f64().unwrap(), hx: m["hx"].as_f64().unwrap(), hz: m["hz"].as_f64().unwrap() };
        let solids: Vec<contact::Obstacle> = c["obstacles"].as_array().unwrap().iter().map(|o| contact::Obstacle { id: o["id"].as_str().unwrap().parse().unwrap(), x0: o["x0"].as_f64().unwrap(), x1: o["x1"].as_f64().unwrap(), z0: o["z0"].as_f64().unwrap(), z1: o["z1"].as_f64().unwrap() }).collect();
        let got = contact::sweep(&mo, &solids);
        let e = &c["expected"];
        let kind = match e["kind"].as_str().unwrap() { "clear" => contact::Kind::Clear, "hit" => contact::Kind::Hit, _ => contact::Kind::Overlap };
        assert_eq!(got.kind, kind, "{}", c["name"]);
        assert_eq!((got.fraction.n.to_string(), got.fraction.d.to_string()), (e["fraction"]["n"].as_str().unwrap().to_string(), e["fraction"]["d"].as_str().unwrap().to_string()), "{}", c["name"]);
        let ids: Vec<u128> = e["ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().parse().unwrap()).collect();
        assert_eq!(got.ids, ids, "{}", c["name"]);
        let normals: Vec<(i32, i32)> = e["normals"].as_array().unwrap().iter().map(|n| (n[0].as_i64().unwrap() as i32, n[1].as_i64().unwrap() as i32)).collect();
        assert_eq!(got.normals, normals, "{}", c["name"]);
        assert!((got.stop.0 - e["stop"]["x"].as_f64().unwrap()).abs() < 1e-12 && (got.stop.1 - e["stop"]["z"].as_f64().unwrap()).abs() < 1e-12, "{}", c["name"]);
    }
}

#[test]
fn district_drive_replays_the_six_portable_cases() {
    use rusttoolz::district::{self, Scene, Start};
    use rusttoolz::motion::{Handling, Pose};
    let Some(r) = read("district-drive-v1.javascript.json") else { return };
    let pose = |p: &serde_json::Value| Pose {
        x: p["x"].as_f64().unwrap(),
        z: p["z"].as_f64().unwrap(),
        yaw: p["yaw"].as_f64().unwrap(),
        speed: p["speed"].as_f64().unwrap(),
        vx: p["vx"].as_f64().unwrap(),
        vz: p["vz"].as_f64().unwrap(),
        steer_vis: p["steer_vis"].as_f64().unwrap(),
        lean: p["lean"].as_f64().unwrap(),
        squat: p["squat"].as_f64().unwrap(),
        braking: p["braking"].as_bool().unwrap(),
    };
    let codes = |v: &serde_json::Value| -> Vec<u8> { v.as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as u8).collect() };
    let diff = |a: &Pose, b: &Pose| {
        [a.x - b.x, a.z - b.z, a.yaw - b.yaw, a.speed - b.speed, a.vx - b.vx, a.vz - b.vz, a.steer_vis - b.steer_vis, a.lean - b.lean, a.squat - b.squat]
            .iter()
            .fold(0.0f64, |m, d| m.max(d.abs()))
            .max(if a.braking == b.braking { 0.0 } else { 1.0 })
    };
    let (mut checked, mut contacts) = (0, 0);
    for case in r["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let sc = &case["scene"];
        let h = &sc["handling"];
        let handling = Handling { speed: h["speed"].as_f64().unwrap(), accel: h["accel"].as_f64().unwrap(), turn: h["turn"].as_f64().unwrap(), grip: h["grip"].as_f64().unwrap(), wet: h["wet"].as_f64().unwrap() };
        let edits: Vec<(u32, u128)> = sc["edits"].as_array().unwrap().iter().map(|e| (e["cell"].as_u64().unwrap() as u32, e["N"].as_str().unwrap().parse().unwrap())).collect();
        let start = if sc["start"] == "wall" { Start::Wall } else { Start::Avenue };
        let scene = Scene::new(sc["district"].as_str().unwrap().parse().unwrap(), &edits, start, case["half"].as_f64().unwrap(), handling).unwrap();
        assert_eq!(diff(&scene.spawn, &pose(&case["initial"])), 0.0, "{name}");
        let mut inputs = codes(&sc["inputs"]);
        if sc["active"] == "branch" {
            let b = &sc["branch"];
            let branch = codes(&b["inputs"]);
            let at = b["fork"].as_u64().unwrap() as usize;
            assert!(district::shares_prefix(&inputs, &branch, at));
            assert_eq!(district::fork(&inputs, at, district::encode(1, 1, false), 160).unwrap(), branch);
            let (a, t) = (scene.trace(&inputs).unwrap(), scene.trace(&branch).unwrap());
            assert_eq!(district::first_difference(&a, &t), Some(at + 1));
            assert!(a.first_contact().is_some() && t.first_contact().is_none());
            inputs = branch;
        }
        let t = scene.trace(&inputs).unwrap();
        for cp in case["checkpoints"].as_array().unwrap() {
            let tick = cp["tick"].as_u64().unwrap() as usize;
            let got = t.frames[tick];
            assert_eq!(diff(&got, &pose(&cp["pose"])), 0.0, "{name} tick {tick}");
            let s = scene.seek(&t, tick).unwrap();
            assert_eq!(s.pose, got);
            assert_eq!(s.card, rusttoolz::district::Card::pack(&inputs[s.checkpoint..(s.checkpoint + rusttoolz::district::EVERY).min(inputs.len())]).unwrap());
            checked += 1;
        }
        let fc = &case["firstContact"];
        match t.first_contact() {
            None => assert!(fc.is_null(), "{name}"),
            Some(r) => {
                assert_eq!(r.tick as u64, fc["tick"].as_u64().unwrap(), "{name}");
                let ids: Vec<u128> = fc["ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().parse().unwrap()).collect();
                assert_eq!(r.contact.ids, ids);
                assert_eq!((r.contact.fraction.n.to_string(), r.contact.fraction.d.to_string()), (fc["fraction"]["n"].as_str().unwrap().to_string(), fc["fraction"]["d"].as_str().unwrap().to_string()));
                assert!((r.stop.0 - fc["stop"]["x"].as_f64().unwrap()).abs() <= 1e-9 && (r.stop.1 - fc["stop"]["z"].as_f64().unwrap()).abs() <= 1e-9);
                contacts += 1;
            }
        }
    }
    assert_eq!((checked, contacts), (36, 3));
}

#[test]
fn the_lookup_fixture_cards_and_candidates() {
    use rusttoolz::contact::{sweep, Kind, Motion};
    use rusttoolz::district::{left_edge, Card, Scene, Start, PROFILE};
    use rusttoolz::motion::Handling;
    let Some(f) = read("district-lookup-v1.javascript.json") else { return };
    assert_eq!(f["profile"], PROFILE);
    let codes = |v: &serde_json::Value| -> Vec<u8> { v.as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as u8).collect() };
    for b in f["blocks"].as_array().unwrap() {
        let input = codes(&b["input"]);
        let card = Card::pack(&input).unwrap();
        assert_eq!(card.length as u64, b["card"]["length"].as_u64().unwrap());
        assert_eq!(card.word.to_decimal(), b["card"]["N"].as_str().unwrap());
        assert_eq!(Card::from_json(&card.to_json()), Some(card));
        assert_eq!(card.commands().unwrap(), codes(&b["commands"]));
        let edge: Vec<u64> = b["edge"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().parse().unwrap()).collect();
        assert_eq!(card.edge(), edge);
        assert_eq!(left_edge(&input.iter().map(|&c| c as u64).collect::<Vec<_>>()), edge);
    }
    let motion = |m: &serde_json::Value| Motion { x: m["x"].as_f64().unwrap(), z: m["z"].as_f64().unwrap(), dx: m["dx"].as_f64().unwrap(), dz: m["dz"].as_f64().unwrap(), hx: m["hx"].as_f64().unwrap(), hz: m["hz"].as_f64().unwrap() };
    let ids = |v: &serde_json::Value| -> Vec<u128> {
        let mut out: Vec<u128> = v.as_array().unwrap().iter().map(|i| i.as_str().unwrap().parse().unwrap()).collect();
        out.sort_unstable();
        out
    };
    let mut cases = 0;
    for n in f["neighborhoods"].as_array().unwrap() {
        let edits: Vec<(u32, u128)> = n["edits"].as_array().unwrap().iter().map(|e| (e["cell"].as_u64().unwrap() as u32, e["N"].as_str().unwrap().parse().unwrap())).collect();
        let scene = Scene::new(n["district"].as_str().unwrap().parse().unwrap(), &edits, Start::Wall, 2.5, Handling::default()).unwrap();
        for c in n["cases"].as_array().unwrap() {
            let mut got: Vec<u128> = scene.lots_near(&motion(&c["motion"])).into_iter().map(|i| scene.solids[i].id).collect();
            got.sort_unstable();
            assert_eq!(got, ids(&c["ids"]));
            cases += 1;
        }
    }
    assert_eq!(cases, 20);
    let w = &f["wallContact"];
    let scene = Scene::new(45, &[], Start::Wall, 2.5, Handling::default()).unwrap();
    let m = motion(&w["motion"]);
    let near = scene.lots_near(&m);
    let solids: Vec<_> = near.iter().map(|&i| scene.solids[i].clone()).collect();
    let r = sweep(&m, &solids);
    assert_eq!(r.kind, Kind::Hit);
    assert_eq!(r.ids, ids(&w["result"]["ids"]));
    assert_eq!(r.fraction.n.to_string(), w["result"]["fraction"]["n"].as_str().unwrap());
}

#[test]
fn moving_pairs_meet_where_the_studio_says() {
    use rusttoolz::contact::{pair, Body, Kind};
    let Some(f) = read("moving-pair-v1.javascript.json") else { return };
    let mut n = 0;
    for case in f["cases"].as_array().unwrap() {
        let r = &case["result"];
        let ids: Vec<u128> = r["ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().parse().unwrap()).collect();
        let b_id: u128 = r["obstacle"]["id"].as_str().unwrap().parse().unwrap();
        let a_id = *ids.iter().find(|&&i| i != b_id).unwrap();
        let cars = case["cars"].as_array().unwrap();
        let fp = r["footprints"].as_array().unwrap();
        let body = |i: usize, id: u128| Body { id, x: cars[i]["x"].as_f64().unwrap(), z: cars[i]["z"].as_f64().unwrap(), dx: cars[i]["dx"].as_f64().unwrap(), dz: cars[i]["dz"].as_f64().unwrap(), hx: fp[i]["hx"].as_f64().unwrap(), hz: fp[i]["hz"].as_f64().unwrap() };
        let p = pair(&body(0, a_id), &body(1, b_id));
        let kind = match r["kind"].as_str().unwrap() { "hit" => Kind::Hit, "clear" => Kind::Clear, _ => Kind::Overlap };
        assert_eq!(p.kind, kind);
        assert_eq!((p.fraction.n.to_string(), p.fraction.d.to_string()), (r["at"]["n"].as_str().unwrap().to_string(), r["at"]["d"].as_str().unwrap().to_string()));
        assert_eq!(p.ids, ids);
        for i in 0..2 {
            let c = &r["contact"][i];
            assert_eq!((p.contact[i].0.n.to_string(), p.contact[i].1.n.to_string()), (c["x"]["n"].as_str().unwrap().to_string(), c["z"]["n"].as_str().unwrap().to_string()));
        }
        n += 1;
    }
    assert_eq!(n, 9);
}

#[test]
fn the_convoy_receipts_replay() {
    use rusttoolz::convoy::*;
    let Some(f) = read("convoy-brake-v1.javascript.json") else { return };
    let mut n = 0;
    for case in f["cases"].as_array().unwrap() {
        let r = &case["receipt"];
        let rc = &r["recipe"];
        let cars = rc["cars"].as_array().unwrap();
        let plan = Plan {
            mode: if rc["mode"] == "throttle" { Mode::Throttle } else { Mode::Coast },
            lead_tick: rc["leadTick"].as_u64().unwrap() as usize,
            delay: rc["delay"].as_u64().unwrap() as usize,
            gap: rc["gap"].as_f64().unwrap(),
            margin: rc["margin"].as_f64().unwrap(),
            top: rc["top"].as_f64().unwrap(),
            accel: rc["accel"].as_f64().unwrap(),
            leader: Car { speed: cars[0]["speed"].as_f64().unwrap(), wet: cars[0]["wet"].as_f64().unwrap(), ..LEADER },
            follower: Car { speed: cars[1]["speed"].as_f64().unwrap(), wet: cars[1]["wet"].as_f64().unwrap(), ..FOLLOWER },
        };
        let got = run(&plan);
        assert!((got.required - r["required"].as_f64().unwrap()).abs() <= 1e-9, "{}", case["name"]);
        assert_eq!(got.fitted, r["fittedGap"].as_f64().unwrap());
        assert_eq!(got.closest_tick as u64, r["closestTick"].as_u64().unwrap());
        for cp in r["checkpoints"].as_array().unwrap() {
            let k = cp["tick"].as_u64().unwrap() as usize;
            assert!((got.reserve_at(k) - cp["reserve"].as_f64().unwrap()).abs() <= 1e-9 && (got.gap_at(k) - cp["gap"].as_f64().unwrap()).abs() <= 1e-9, "{} tick {k}", case["name"]);
        }
        match (&got.contact, r["contact"].is_null()) {
            (None, true) => {}
            (Some(c), false) => assert_eq!((c.tick as u64, c.fraction.n.to_string()), (r["contact"]["tick"].as_u64().unwrap(), r["contact"]["fraction"]["n"].as_str().unwrap().to_string())),
            _ => panic!("{}: contact mismatch", case["name"]),
        }
        n += 1;
    }
    assert_eq!(n, 8);
}

#[test]
fn the_zipper_receipts_replay() {
    use rusttoolz::zipper::*;
    let Some(f) = read("zipper-gate-v1.javascript.json") else { return };
    let mut n = 0;
    for case in f["cases"].as_array().unwrap() {
        let r = &case["receipt"];
        let rc = &r["recipe"];
        let lane = |i: usize| -> Vec<(u128, i64)> { rc["lanes"][i].as_array().unwrap().iter().map(|c| (c["card"]["N"].as_str().unwrap().parse().unwrap(), c["ready"].as_i64().unwrap())).collect() };
        let (a, b) = (lane(0), lane(1));
        let gate = Gate {
            start_lane: rc["startLane"].as_u64().unwrap() as usize,
            policy: if rc["policy"] == "arrival" { Policy::Arrival } else { Policy::Zipper },
            speed: rc["speed"].as_f64().unwrap(),
            gate_ticks: rc["gateTicks"].as_i64().unwrap(),
            gap_mode: if rc["gapMode"] == "braking" { GapMode::Braking } else { GapMode::Fixed },
            gap: rc["gap"].as_f64().unwrap(),
            delay: rc["delay"].as_u64().unwrap() as usize,
            wet: rc["wet"].as_f64().unwrap(),
            margin: rc["margin"].as_f64().unwrap(),
        };
        let got = run(&Plan::new([&a, &b], gate, &[]).unwrap());
        assert_eq!((got.gap, got.stride, got.end), (r["gap"].as_f64().unwrap(), r["stride"].as_i64().unwrap(), r["end"].as_i64().unwrap()), "{}", case["name"]);
        for (e, w) in got.events.iter().zip(r["events"].as_array().unwrap()) {
            assert_eq!((e.label(), e.tick, e.address.to_string()), (w["label"].as_str().unwrap().to_string(), w["tick"].as_i64().unwrap(), w["address"].as_str().unwrap().to_string()));
            if let Some(g) = &e.pair_gap {
                assert_eq!(g.n.to_string(), w["pairGap"]["n"].as_str().unwrap());
            }
        }
        n += 1;
    }
    assert_eq!(n, 8);
}
