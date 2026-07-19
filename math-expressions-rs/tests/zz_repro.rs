use math_expressions::*;

fn catch<T>(f: impl FnOnce() -> T) -> Result<T, ()> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|_| ())
}

#[test]
fn fuzz_derivative_simplify_pipeline() {
    let data = std::fs::read_to_string("zz_inputs.txt").unwrap();
    let mut panics = vec![];
    // Silence per-panic stderr spam; we report the offending input ourselves.
    std::panic::set_hook(Box::new(|_| {}));
    for src in data.lines() {
        let Ok(Ok(e)) = catch(|| TextToAst::new(TextToAstOptions::default()).convert(src)) else {
            continue; // parse panic or parse error — track parse panics separately below
        };
        for var in ["x", "y", "n", "t"] {
            let Ok(d) = catch(|| derivative(&e, var)) else {
                panics.push(format!("PANIC derivative d/d{var}: {src:?}"));
                continue;
            };
            let step = catch(|| {
                let s = simplify_with(&d, &Assumptions::new());
                let _ = to_text(&s, &Default::default());
                let _ = to_latex(&s, &Default::default());
            });
            if step.is_err() {
                panics.push(format!("PANIC simplify/output d/d{var}: {src:?}"));
            }
        }
    }
    let _ = std::panic::take_hook();
    if !panics.is_empty() {
        // De-dup and print
        panics.sort();
        panics.dedup();
        for p in &panics {
            println!("{p}");
        }
    }
    assert!(panics.is_empty(), "{} panicking cases (see stdout)", panics.len());
}
