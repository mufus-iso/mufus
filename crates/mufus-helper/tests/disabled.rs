use std::process::Command;

#[test]
fn helper_rejects_operations() -> Result<(), Box<dyn std::error::Error>> {
    let result = Command::new(env!("CARGO_BIN_EXE_mufus-helper"))
        .arg("write")
        .output()?;
    assert!(!result.status.success());
    Ok(())
}
