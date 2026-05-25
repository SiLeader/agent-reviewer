// Copyright 2026- SiLeader (Cerussite).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver, ServiceTargetResolverFn};
use genai::{ModelIden, ModelName, ServiceTarget};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct ModelProvider {
    models: HashMap<ModelName, ServiceTargetOpt>,
}

#[derive(Debug, Clone)]
pub(crate) struct ServiceTargetOpt {
    pub(crate) endpoint: Option<Endpoint>,
    pub(crate) auth: Option<AuthData>,
    pub(crate) model: Option<ModelIden>,
}

impl ServiceTargetOpt {
    fn or(&self, target: ServiceTarget) -> ServiceTarget {
        ServiceTarget {
            endpoint: self.endpoint.clone().unwrap_or(target.endpoint),
            auth: self.auth.clone().unwrap_or(target.auth),
            model: self.model.clone().unwrap_or(target.model),
        }
    }
}

impl ModelProvider {
    pub(crate) fn create_resolver(
        models: HashMap<ModelName, ServiceTargetOpt>,
    ) -> ServiceTargetResolver {
        ServiceTargetResolver::ResolverFn(Arc::new(Box::new(ModelProvider { models })))
    }
}

impl ServiceTargetResolverFn for ModelProvider {
    fn exec_fn(&self, service_target: ServiceTarget) -> genai::resolver::Result<ServiceTarget> {
        let model = self
            .models
            .get(&service_target.model.model_name)
            .ok_or_else(|| {
                genai::resolver::Error::Custom(format!(
                    "Model not found: {}",
                    service_target.model.model_name
                ))
            })?;
        Ok(model.or(service_target))
    }

    fn clone_box(&self) -> Box<dyn ServiceTargetResolverFn> {
        Box::new(self.clone())
    }
}
