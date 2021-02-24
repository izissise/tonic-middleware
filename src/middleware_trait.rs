/// Define tonic middlewares trait

#[tonic::async_trait]
pub trait TonicMiddleware {
    /// Middleware's added context to request
    type ReqContext;
    /// Error type that `before` can return
    type Error;

    /// Called before the request is processed by the service
    /// receive requests, and can exit early on errors
    async fn before<T: Send + Sync>(
        &self,
        request: &tonic::Request<T>,
    ) -> Result<Self::ReqContext, Self::Error>;

    /// Called after the request was processed by the service
    /// receive context and responses
    async fn after<T: Send + Sync>(
        &self,
        context: Self::ReqContext,
        response: &Result<tonic::Response<T>, tonic::Status>,
    );
}
