/*-
 * ========================LICENSE_START=================================
 * PREvant REST API
 * %%
 * Copyright (C) 2018 - 2019 aixigo AG
 * %%
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 * =========================LICENSE_END==================================
 */

use super::traefik::TraefikIngressRoute;
use crate::config::ContainerConfig;
use crate::deployment::DeploymentUnit;
use crate::models::service::{Service, ServiceStatus};
use crate::models::{AppName, ContainerType, ServiceConfig};
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use failure::Error;
use multimap::MultiMap;

#[async_trait]
pub trait Infrastructure: Send + Sync {
    /// Returns a `MultiMap` of `app-name` and the running services for this app.
    async fn get_services(&self) -> Result<MultiMap<AppName, Service>, Error>;

    /// Deploys the services of the given set of `ServiceConfig`.
    ///
    /// The implementation must ensure that:
    /// - the services are able to communicate with each other with the service name. For example,
    ///   they must be able the execute `ping <service_name>`.
    /// - the services must be deployed once. If a service is already running, it must be redeployed.
    /// - the services must be discoverable for further calls. For example, `self.stop_services(...)`
    ///   must be able to find the corresponding services.
    async fn deploy_services(
        &self,
        status_id: &str,
        deployment_unit: &DeploymentUnit,
        container_config: &ContainerConfig,
    ) -> Result<Vec<Service>, Error>;

    async fn get_status_change(&self, _status_id: &str) -> Result<Option<Vec<Service>>, Error> {
        Ok(None)
    }

    /// Stops the services running for the given `app_name`
    ///
    /// The implementation must ensure that it returns the services that have been
    /// stopped.
    async fn stop_services(
        &self,
        status_id: &str,
        app_name: &AppName,
    ) -> Result<Vec<Service>, Error>;

    /// Returns the log lines with a the corresponding timestamps in it.
    async fn get_logs(
        &self,
        app_name: &AppName,
        service_name: &str,
        from: &Option<DateTime<FixedOffset>>,
        limit: usize,
    ) -> Result<Option<Vec<(DateTime<FixedOffset>, String)>>, Error>;

    /// Changes the status of a service, for example, the service might me stopped or started.
    async fn change_status(
        &self,
        app_name: &AppName,
        service_name: &str,
        status: ServiceStatus,
    ) -> Result<Option<Service>, Error>;

    /// Determines the [router rule](https://doc.traefik.io/traefik/routing/routers/) that points
    /// to PREvant it self so services will be reachable on the same route, e.g. host name.
    async fn base_traefik_ingress_route(&self) -> Result<Option<TraefikIngressRoute>, Error> {
        Ok(None)
    }

    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        panic!("This should be only use in test environments with following approach: https://stackoverflow.com/a/33687996/5088458")
    }
}

impl dyn Infrastructure {
    /// Returns the configuration of all services running for the given application name.
    pub async fn get_configs_of_app(
        &self,
        app_name: &AppName,
    ) -> Result<Vec<ServiceConfig>, Error> {
        let services = self.get_services().await?;
        Ok(services
            .get_vec(app_name)
            .map_or_else(Vec::new, |services| {
                services
                    .iter()
                    .filter(|service| {
                        *service.container_type() == ContainerType::Instance
                            || *service.container_type() == ContainerType::Replica
                    })
                    .map(|service| service.config().clone())
                    .collect()
            }))
    }
}
