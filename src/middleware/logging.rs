// 请求日志中间件
// 记录API请求日志，包括请求时间、响应时间、状态码等

use actix_web::{
    dev::{ServiceRequest, ServiceResponse, Transform},
    Error, Result as ActixResult,
};
use futures_util::future::{ok, Ready};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use std::time::Instant;

/// 请求日志中间件
pub struct RequestLogging;

impl<S, B> Transform<S, ServiceRequest> for RequestLogging
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestLoggingMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestLoggingMiddleware { service })
    }
}

pub struct RequestLoggingMiddleware<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for RequestLoggingMiddleware<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();
        let remote_addr = req.connection_info().realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        let fut = self.service.call(req);

        Box::pin(async move {
            let result = fut.await;
            let duration = start_time.elapsed();

            match &result {
                Ok(response) => {
                    let status = response.status().as_u16();
                    
                    if status >= 400 {
                        log::warn!(
                            "{} {} {} {}ms - {}",
                            remote_addr, method, path, duration.as_millis(), status
                        );
                    } else {
                        log::info!(
                            "{} {} {} {}ms - {}",
                            remote_addr, method, path, duration.as_millis(), status
                        );
                    }
                },
                Err(e) => {
                    log::error!(
                        "{} {} {} {}ms - ERROR: {}",
                        remote_addr, method, path, duration.as_millis(), e
                    );
                }
            }

            result
        })
    }
}
