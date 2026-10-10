use rcgen::{BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

pub struct CaBundle {
    pub ca_cert_der: CertificateDer<'static>,
    pub ca_cert_pem: String,
    key_pair: KeyPair,
    params: CertificateParams,
}

pub struct IssuedCert {
    pub cert_der: CertificateDer<'static>,
    pub cert_pem: String,
    pub key_der: PrivateKeyDer<'static>,
    pub key_pem: String,
}

impl CaBundle {
    /// Load existing CA from directory, or generate new one
    pub fn load_or_create(dir: &str) -> anyhow::Result<Self> {
        let ca_pem_path = format!("{dir}/ca.pem");
        let ca_key_path = format!("{dir}/ca-key.pem");

        if std::path::Path::new(&ca_pem_path).exists() && std::path::Path::new(&ca_key_path).exists() {
            let ca_pem = std::fs::read_to_string(&ca_pem_path)?;
            let ca_key_pem = std::fs::read_to_string(&ca_key_path)?;
            let key_pair = KeyPair::from_pem(&ca_key_pem)?;

            let mut params = CertificateParams::default();
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            params.key_usages.push(KeyUsagePurpose::KeyCertSign);
            params.key_usages.push(KeyUsagePurpose::CrlSign);
            let mut dn = DistinguishedName::new();
            dn.push(DnType::CommonName, "Pulse CA");
            dn.push(DnType::OrganizationName, "Pulse Contributors");
            params.distinguished_name = dn;

            let ca_cert_der = CertificateDer::from(pem_to_der(&ca_pem)?);

            Ok(Self { ca_cert_der, ca_cert_pem: ca_pem, key_pair, params })
        } else {
            let bundle = Self::new()?;
            // Save CA key for future loads
            std::fs::write(&ca_key_path, bundle.key_pair.serialize_pem())?;
            Ok(bundle)
        }
    }

    pub fn new() -> anyhow::Result<Self> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages.push(KeyUsagePurpose::KeyCertSign);
        params.key_usages.push(KeyUsagePurpose::CrlSign);
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Pulse CA");
        dn.push(DnType::OrganizationName, "Pulse Contributors");
        params.distinguished_name = dn;

        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;

        let ca_cert_der = CertificateDer::from(cert.der().to_vec());
        let ca_cert_pem = cert.pem();

        Ok(Self { ca_cert_der, ca_cert_pem, key_pair, params })
    }

    pub fn ca_key_pem(&self) -> String {
        self.key_pair.serialize_pem()
    }

    pub fn issue_server_cert(&self, san: &[&str]) -> anyhow::Result<IssuedCert> {
        self.issue_cert(san, false)
    }

    pub fn issue_client_cert(&self, cn: &str) -> anyhow::Result<IssuedCert> {
        self.issue_cert(&[cn], true)
    }

    fn issue_cert(&self, names: &[&str], is_client: bool) -> anyhow::Result<IssuedCert> {
        let mut params = CertificateParams::new(names.iter().map(|s| s.to_string()).collect::<Vec<_>>())?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, names[0]);
        params.distinguished_name = dn;

        if is_client {
            params.extended_key_usages.push(rcgen::ExtendedKeyUsagePurpose::ClientAuth);
        } else {
            params.extended_key_usages.push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);
        }

        let issuer = Issuer::from_params(&self.params, &self.key_pair);
        let key_pair = KeyPair::generate()?;
        let cert = params.signed_by(&key_pair, &issuer)?;

        let cert_der = CertificateDer::from(cert.der().to_vec());
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

        Ok(IssuedCert { cert_der, cert_pem, key_der, key_pem })
    }
}

fn pem_to_der(pem: &str) -> anyhow::Result<Vec<u8>> {
    // Simple base64 decode of PEM content (between -----BEGIN/END lines)
    let encoded: String = pem.lines().filter(|l| !l.starts_with("-----") && !l.is_empty()).collect();
    let mut buf = Vec::with_capacity(encoded.len());
    // Decode base64 manually using a lookup table
    static B64: &[u8; 128] = &{
        let mut table = [255u8; 128];
        let mut i = 0u8;
        while i < 26 {
            table[(b'A' + i) as usize] = i;
            table[(b'a' + i) as usize] = i + 26;
            i += 1;
        }
        let mut i = 0u8;
        while i < 10 {
            table[(b'0' + i) as usize] = i + 52;
            i += 1;
        }
        table[b'+' as usize] = 62;
        table[b'/' as usize] = 63;
        table
    };
    let bytes = encoded.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let (a, b, c, d) = (
            B64[bytes[i] as usize] as u32,
            B64[bytes[i + 1] as usize] as u32,
            B64[bytes[i + 2] as usize] as u32,
            B64[bytes[i + 3] as usize] as u32,
        );
        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        buf.push((triple >> 16) as u8);
        if bytes[i + 2] != b'=' {
            buf.push((triple >> 8) as u8);
        }
        if bytes[i + 3] != b'=' {
            buf.push(triple as u8);
        }
        i += 4;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ca_generates_valid_bundle() {
        let ca = CaBundle::new().unwrap();
        assert!(ca.ca_cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(!ca.ca_cert_der.is_empty());
    }

    #[test]
    fn test_issue_server_cert() {
        let ca = CaBundle::new().unwrap();
        let cert = ca.issue_server_cert(&["localhost", "127.0.0.1"]).unwrap();
        assert!(cert.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(cert.key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(!cert.cert_der.is_empty());
    }

    #[test]
    fn test_issue_client_cert() {
        let ca = CaBundle::new().unwrap();
        let cert = ca.issue_client_cert("test-agent").unwrap();
        assert!(cert.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(cert.key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_different_agents_get_different_certs() {
        let ca = CaBundle::new().unwrap();
        let c1 = ca.issue_client_cert("agent-1").unwrap();
        let c2 = ca.issue_client_cert("agent-2").unwrap();
        assert_ne!(c1.cert_pem, c2.cert_pem);
        assert_ne!(c1.key_pem, c2.key_pem);
    }
}
