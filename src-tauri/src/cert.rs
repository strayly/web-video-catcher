

use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
use std::fs;
use std::path::Path;
use std::process::Command;


pub fn generate_ca() -> Result<(String, Vec<u8>), String> {
    let mut params = CertificateParams::new(vec!["网页视频捕手 CA".to_string()])
        .map_err(|e| e.to_string())?;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let key_pair = KeyPair::generate().map_err(|e| e.to_string())?;
    let cert = params.self_signed(&key_pair).map_err(|e| e.to_string())?;
    let pem = cert.pem();
    let der = cert.der().to_vec();
    Ok((pem, der))
}


pub fn ensure_ca(ca_dir: &Path) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    fs::create_dir_all(ca_dir).map_err(|e| e.to_string())?;
    let pem_path = ca_dir.join("ca.pem");
    let der_path = ca_dir.join("ca.der");
    if !pem_path.exists() {
        let (pem, der) = generate_ca()?;
        fs::write(&pem_path, pem).map_err(|e| e.to_string())?;
        fs::write(&der_path, der).map_err(|e| e.to_string())?;
    }
    Ok((pem_path, der_path))
}


pub fn install_ca_windows(der_path: &Path) -> Result<(), String> {
    let out = Command::new("certutil")
        .args(["-addstore", "-f", "Root", &der_path.to_string_lossy()])
        .output()
        .map_err(|e| format!("无法启动 certutil: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}
