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
    pub(crate) fn new(models: HashMap<ModelName, ServiceTargetOpt>) -> ServiceTargetResolver {
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
