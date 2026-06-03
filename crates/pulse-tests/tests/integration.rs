use pulse_master::ca::CaBundle;
use pulse_master::service::PulseService;
use pulse_proto::pb::{
    CpuMetrics, MemoryMetrics, MetricsPayload, RegisterRequest, SystemMetrics, pulse_agent_client::PulseAgentClient,
};
use tokio::net::TcpListener;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity, Server, ServerTlsConfig};

async fn spawn_server_plain() -> String {
    let listener = TcpListener::bind("[::1]:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    tokio::spawn(async move {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder().add_service(PulseService::new().into_server()).serve_with_incoming(incoming).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    url
}

#[tokio::test]
async fn test_register_plain() {
    let url = spawn_server_plain().await;
    let mut client = PulseAgentClient::connect(url).await.unwrap();

    let resp = client
        .register(RegisterRequest {
            hostname: "test-host".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            agent_version: "0.1.0".into(),
            labels: vec!["test".into()],
        })
        .await
        .unwrap();

    let r = resp.into_inner();
    assert!(r.agent_id.starts_with("agent-"));
    assert_eq!(r.metrics_interval_ms, 10_000);
}

#[tokio::test]
async fn test_stream_metrics_plain() {
    let url = spawn_server_plain().await;
    let mut client = PulseAgentClient::connect(url).await.unwrap();

    let payloads = vec![
        MetricsPayload {
            timestamp_ms: 1000,
            agent_id: "agent-0001".into(),
            cpu: Some(CpuMetrics {
                usage_per_core_bp: vec![2500, 3000],
                usage_total_bp: 2750,
                load_avg_1m_cp: 150,
                load_avg_5m_cp: 120,
                load_avg_15m_cp: 100,
            }),
            memory: Some(MemoryMetrics {
                total_bytes: 8_000_000_000,
                used_bytes: 4_000_000_000,
                available_bytes: 4_000_000_000,
                swap_total_bytes: 0,
                swap_used_bytes: 0,
                cgroup_limit_bytes: 0,
                cgroup_usage_bytes: 0,
            }),
            disks: vec![],
            net: vec![],
            system: Some(SystemMetrics { uptime_secs: 3600, processes_total: 150, processes_running: 3 }),
        },
        MetricsPayload {
            timestamp_ms: 11000,
            agent_id: "agent-0001".into(),
            cpu: Some(CpuMetrics { usage_per_core_bp: vec![5000, 6000], usage_total_bp: 5500, ..Default::default() }),
            memory: None,
            disks: vec![],
            net: vec![],
            system: None,
        },
    ];

    let stream = tokio_stream::iter(payloads);
    let resp = client.stream_metrics(stream).await.unwrap();
    let ack = resp.into_inner();
    assert!(ack.ok);
    assert_eq!(ack.next_interval_ms, 10_000);
}

#[tokio::test]
async fn test_register_mtls() {
    let ca = CaBundle::new().unwrap();
    let server_cert = ca.issue_server_cert(&["localhost", "::1"]).unwrap();
    let client_cert = ca.issue_client_cert("test-agent").unwrap();

    let identity = Identity::from_pem(&server_cert.cert_pem, &server_cert.key_pem);
    let client_ca_cert = Certificate::from_pem(&ca.ca_cert_pem);

    let tls_server = ServerTlsConfig::new().identity(identity).client_ca_root(client_ca_cert.clone());

    let listener = TcpListener::bind("[::1]:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .tls_config(tls_server)
            .unwrap()
            .add_service(PulseService::new().into_server())
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client_identity = Identity::from_pem(&client_cert.cert_pem, &client_cert.key_pem);
    let tls_client =
        ClientTlsConfig::new().domain_name("localhost").ca_certificate(client_ca_cert).identity(client_identity);

    let channel = Channel::from_shared(format!("https://[::1]:{port}"))
        .unwrap()
        .tls_config(tls_client)
        .unwrap()
        .connect()
        .await
        .unwrap();

    let mut client = PulseAgentClient::new(channel);

    let resp = client
        .register(RegisterRequest {
            hostname: "mtls-host".into(),
            os: "linux".into(),
            arch: "aarch64".into(),
            agent_version: "0.1.0".into(),
            labels: vec!["secure".into()],
        })
        .await
        .unwrap();

    let r = resp.into_inner();
    assert!(r.agent_id.starts_with("agent-"));
}

#[tokio::test]
async fn test_mtls_rejects_no_client_cert() {
    let ca = CaBundle::new().unwrap();
    let server_cert = ca.issue_server_cert(&["localhost", "::1"]).unwrap();

    let identity = Identity::from_pem(&server_cert.cert_pem, &server_cert.key_pem);
    let client_ca_cert = Certificate::from_pem(&ca.ca_cert_pem);

    let tls_server = ServerTlsConfig::new().identity(identity).client_ca_root(client_ca_cert.clone());

    let listener = TcpListener::bind("[::1]:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .tls_config(tls_server)
            .unwrap()
            .add_service(PulseService::new().into_server())
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Connect WITHOUT client cert — should fail
    let tls_client = ClientTlsConfig::new().domain_name("localhost").ca_certificate(client_ca_cert);

    let result =
        Channel::from_shared(format!("https://[::1]:{port}")).unwrap().tls_config(tls_client).unwrap().connect().await;

    // Connection may succeed but RPC should fail
    if let Ok(channel) = result {
        let mut client = PulseAgentClient::new(channel);
        let resp = client
            .register(RegisterRequest {
                hostname: "rejected".into(),
                os: "linux".into(),
                arch: "x86_64".into(),
                agent_version: "0.1.0".into(),
                labels: vec![],
            })
            .await;
        assert!(resp.is_err(), "mTLS should reject client without cert");
    }
    // If connect itself failed, that's also valid
}

#[tokio::test]
async fn test_mtls_rejects_wrong_ca() {
    let ca = CaBundle::new().unwrap();
    let wrong_ca = CaBundle::new().unwrap();
    let server_cert = ca.issue_server_cert(&["localhost", "::1"]).unwrap();
    let wrong_client_cert = wrong_ca.issue_client_cert("evil-agent").unwrap();

    let identity = Identity::from_pem(&server_cert.cert_pem, &server_cert.key_pem);
    let client_ca_cert = Certificate::from_pem(&ca.ca_cert_pem);

    let tls_server = ServerTlsConfig::new().identity(identity).client_ca_root(client_ca_cert.clone());

    let listener = TcpListener::bind("[::1]:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .tls_config(tls_server)
            .unwrap()
            .add_service(PulseService::new().into_server())
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let wrong_identity = Identity::from_pem(&wrong_client_cert.cert_pem, &wrong_client_cert.key_pem);
    let tls_client =
        ClientTlsConfig::new().domain_name("localhost").ca_certificate(client_ca_cert).identity(wrong_identity);

    let result =
        Channel::from_shared(format!("https://[::1]:{port}")).unwrap().tls_config(tls_client).unwrap().connect().await;

    if let Ok(channel) = result {
        let mut client = PulseAgentClient::new(channel);
        let resp = client
            .register(RegisterRequest {
                hostname: "evil".into(),
                os: "linux".into(),
                arch: "x86_64".into(),
                agent_version: "0.1.0".into(),
                labels: vec![],
            })
            .await;
        assert!(resp.is_err(), "mTLS should reject cert signed by wrong CA");
    }
}
