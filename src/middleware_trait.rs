/// Define tonic middlewares trait

#[tonic::async_trait]
pub trait TonicMiddleware {
    type ReqEnv;
    type Error;

    /// Called before the request is processed by the servicer
    async fn before_request<T: Send + Sync>(
        &self,
        request: &tonic::Request<T>,
    ) -> Result<Self::ReqEnv, Self::Error>;

    /// Called after the request was processed by the servicer
    async fn after_request<T: Send + Sync>(
        &self,
        env: Self::ReqEnv,
        response: &Result<tonic::Response<T>, tonic::Status>,
    );
}
