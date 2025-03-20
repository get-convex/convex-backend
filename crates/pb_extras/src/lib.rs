use tonic::server::NamedService;

pub trait ReflectionService: NamedService {
    /// The list of methods supported by this service, e.g. "ExecuteVectorQuery"
    const METHODS: &[&str];
}
