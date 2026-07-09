use std::path::Path;
use std::process::Command;

/// E2E test: invoke the CLI binary against the test data.
#[test]
fn test_cli_font_norm() {
    let out_dir = temp_out_dir("font_norm");

    let status = Command::new(env_binary())
        .arg("tests/in/font_norm.bmpf")
        .arg("tests/in/font_norm.bmp")
        .arg(&out_dir)
        .status()
        .expect("failed to launch bmpf2fnt binary");
    assert!(status.success());

    let fnt_path = Path::new(&out_dir).join("font_norm.fnt");
    assert!(fnt_path.exists(), "font_norm.fnt not created");

    let contents = std::fs::read_to_string(&fnt_path).unwrap();
    assert!(contents.contains(r#"face="font_norm.bmp""#));
    assert!(contents.contains("scaleW=1616 scaleH=19"));
    assert!(contents.contains("chars count=103"));

    std::fs::remove_dir_all(&out_dir).ok();
}

#[test]
fn test_cli_font_tiny() {
    let out_dir = temp_out_dir("font_tiny");

    let status = Command::new(env_binary())
        .arg("tests/in/font_tiny.bmpf")
        .arg("tests/in/font_tiny.bmp")
        .arg(&out_dir)
        .status()
        .expect("failed to launch bmpf2fnt binary");
    assert!(status.success());

    let fnt_path = Path::new(&out_dir).join("font_tiny.fnt");
    assert!(fnt_path.exists(), "font_tiny.fnt not created");

    let contents = std::fs::read_to_string(&fnt_path).unwrap();
    assert!(contents.contains(r#"face="font_tiny.bmp""#));
    assert!(contents.contains("lineHeight=13 base=13"));
    assert!(contents.contains("chars count=256"));

    std::fs::remove_dir_all(&out_dir).ok();
}

#[test]
fn test_cli_multiple_bmps() {
    let out_dir = temp_out_dir("font_multiple");

    // font_norm.bmpf should work for all font_norm*.bmp textures
    let status = Command::new(env_binary())
        .arg("tests/in/font_norm.bmpf")
        .arg("tests/in/font_norm.bmp")
        .arg("tests/in/font_norm_good.bmp")
        .arg("tests/in/font_norm_bad.bmp")
        .arg(&out_dir)
        .status()
        .expect("failed to launch bmpf2fnt binary");
    assert!(status.success());

    for name in &["font_norm", "font_norm_good", "font_norm_bad"] {
        let fnt_path = Path::new(&out_dir).join(format!("{name}.fnt"));
        assert!(fnt_path.exists(), "{name}.fnt not created");
        let contents = std::fs::read_to_string(&fnt_path).unwrap();
        assert!(contents.contains(&format!(r#"face="{name}.bmp""#)));
        assert!(contents.contains("scaleW=1616 scaleH=19"));
    }

    std::fs::remove_dir_all(&out_dir).ok();
}

#[test]
fn test_cli_help() {
    let out = Command::new(env_binary())
        .arg("--help")
        .output()
        .expect("failed to launch bmpf2fnt binary");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("<bmpf>"));

    // Also test -?
    let out2 = Command::new(env_binary())
        .arg("-?")
        .output()
        .expect("failed to launch bmpf2fnt binary");
    assert!(out2.status.success());
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(stdout2.contains("Usage:"));
}

/// Locate the built binary.
fn env_binary() -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!("target/{profile}/bmpf2fnt")
}

/// Create a temporary output directory.
fn temp_out_dir(label: &str) -> String {
    let dir = format!("/tmp/bmpf2fnt_e2e_{label}");
    std::fs::create_dir_all(&dir).ok();
    dir
}
