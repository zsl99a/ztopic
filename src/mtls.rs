use std::{io::Cursor, path::Path};

use s2n_quic::provider::tls::{
    default::{rustls, Certificate, Client, PrivateKey, Server},
    Provider,
};

/// 默认密码套件
static DEFAULT_CIPHER_SUITES: &[rustls::SupportedCipherSuite] = &[
    rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
    rustls::cipher_suite::TLS13_AES_128_GCM_SHA256,
    rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
];
/// 默认协议版本
static DEFAULT_PROTOCOL_VERSIONS: &[&rustls::SupportedProtocolVersion] = &[&rustls::version::TLS13];

/// mutual TLS 提供者
pub struct MtlsProvider {
    root_store: rustls::RootCertStore,
    my_cert_chain: Vec<Certificate>,
    my_private_key: PrivateKey,
}

impl MtlsProvider {
    pub async fn new<A, B, C>(ca_cert_pem: A, my_cert_pem: B, my_key_pem: C) -> Result<Self, rustls::Error>
    where
        A: AsRef<Path>,
        B: AsRef<Path>,
        C: AsRef<Path>,
    {
        Ok(Self {
            root_store: into_root_store(ca_cert_pem.as_ref()).await?,
            my_cert_chain: into_certificates(my_cert_pem.as_ref()).await?,
            my_private_key: into_private_key(my_key_pem.as_ref()).await?,
        })
    }
}

impl Provider for MtlsProvider {
    type Server = Server;

    type Client = Client;

    type Error = rustls::Error;

    fn start_server(self) -> Result<Self::Server, Self::Error> {
        let mut config = rustls::ServerConfig::builder()
            .with_cipher_suites(DEFAULT_CIPHER_SUITES)
            .with_safe_default_kx_groups()
            .with_protocol_versions(DEFAULT_PROTOCOL_VERSIONS)?
            .with_client_cert_verifier(rustls::server::AllowAnyAuthenticatedClient::new(self.root_store))
            .with_single_cert(self.my_cert_chain, self.my_private_key)?;
        config.ignore_client_order = true;
        config.alpn_protocols = vec![b"plk.1".to_vec()];
        Ok(Server::new(config))
    }

    fn start_client(self) -> Result<Self::Client, Self::Error> {
        let mut config = rustls::ClientConfig::builder()
            .with_cipher_suites(DEFAULT_CIPHER_SUITES)
            .with_safe_default_kx_groups()
            .with_protocol_versions(DEFAULT_PROTOCOL_VERSIONS)?
            .with_root_certificates(self.root_store)
            .with_single_cert(self.my_cert_chain, self.my_private_key)?;
        config.alpn_protocols = vec![b"plk.1".to_vec()];
        Ok(Client::new(config))
    }
}

/// 获取根证书
async fn into_root_store(path: &Path) -> Result<rustls::RootCertStore, rustls::Error> {
    let mut root_store = rustls::RootCertStore::empty();
    let certs = into_certificates(path).await?;
    for cert in certs {
        root_store
            .add(&cert)
            .map_err(|_| rustls::Error::General("Failed to load CA certificate".into()))?;
    }
    Ok(root_store)
}

/// 获取证书链
async fn into_certificates(path: &Path) -> Result<Vec<Certificate>, rustls::Error> {
    let pemfile = into_pemfile(path).await?;
    let certs = rustls_pemfile::certs(&mut &pemfile[..]).map_err(|_| rustls::Error::General("Failed to load certificate chain".into()))?;
    Ok(certs.into_iter().map(Certificate).collect())
}

/// 获取私钥
async fn into_private_key(path: &Path) -> Result<PrivateKey, rustls::Error> {
    let pemfile = into_pemfile(path).await?;
    let parsers = [rustls_pemfile::rsa_private_keys, rustls_pemfile::pkcs8_private_keys];
    let mut cursor = Cursor::new(&pemfile);
    for parser in parsers.iter() {
        cursor.set_position(0);
        match parser(&mut cursor) {
            Ok(ref keys) if keys.is_empty() => continue,
            Ok(mut keys) => {
                if keys.len() != 1 {
                    return Err(rustls::Error::General("Multiple private keys found".into()));
                }
                return Ok(PrivateKey(keys.remove(0)));
            }
            Err(_) => continue,
        }
    }
    Err(rustls::Error::General("Failed to load private key".into()))
}

/// 获取 PEM 文件
async fn into_pemfile(path: &Path) -> Result<Vec<u8>, rustls::Error> {
    tokio::fs::read(path)
        .await
        .map_err(|e| rustls::Error::General(format!("Failed to read {:?}: {}", path, e)))
}
