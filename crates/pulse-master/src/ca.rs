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
    pub fn new() -> anyhow::Result<Self> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages.push(KeyUsagePurpose::KeyCertSign);
        params.key_usages.push(KeyUsagePurpose::CrlSign);
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Vixpi Pulse CA");
        dn.push(DnType::OrganizationName, "GCN Solucoes");
        params.distinguished_name = dn;

        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;

        let ca_cert_der = CertificateDer::from(cert.der().to_vec());
        let ca_cert_pem = cert.pem();

        Ok(Self { ca_cert_der, ca_cert_pem, key_pair, params })
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
