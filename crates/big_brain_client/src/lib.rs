use big_brain_private_api_types::{
    DeploymentAuthPreviewArgs,
    DeploymentAuthProdArgs,
    DeploymentAuthResponse,
    DeploymentAuthWithinCurrentProjectArgs,
    TeamAndProjectForDeploymentResponse,
};
use common::types::ProjectId;

pub struct BigBrainClient {
    provision_host: String,
    access_token: String,
}

impl BigBrainClient {
    pub fn new(provision_host: String, access_token: String) -> Self {
        Self {
            provision_host,
            access_token,
        }
    }

    pub async fn get_project_and_team_for_deployment(
        &self,
        deployment_name: String,
    ) -> anyhow::Result<TeamAndProjectForDeploymentResponse> {
        let client = reqwest::Client::new();
        let host = &self.provision_host;
        let url = format!("{host}/api/deployment/{deployment_name}/team_and_project");
        let resp = client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!(anyhow::anyhow!(e)
                .context(format!("delete_project failed: {}", resp.text().await?)));
        }
        Ok(resp.json().await?)
    }

    pub async fn delete_project(&self, project_id: ProjectId) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let host = &self.provision_host;
        let url = format!("{host}/api/dashboard/delete_project/{project_id}");
        let resp = client
            .post(url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!(anyhow::anyhow!(e)
                .context(format!("delete_project failed: {}", resp.text().await?)));
        }
        Ok(())
    }

    pub async fn deployment_credentials(
        &self,
        args: DeploymentAuthWithinCurrentProjectArgs,
    ) -> anyhow::Result<DeploymentAuthResponse> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/api/deployment/authorize_within_current_project",
            self.provision_host
        );
        let resp = client
            .post(url)
            .bearer_auth(&self.access_token)
            .json(&args)
            .send()
            .await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!(anyhow::anyhow!(e).context(format!(
                "deployment_credentials failed: {}",
                resp.text().await?
            )));
        }
        Ok(resp.json::<DeploymentAuthResponse>().await?)
    }

    pub async fn preview_deployment_credentials(
        &self,
        args: DeploymentAuthPreviewArgs,
    ) -> anyhow::Result<DeploymentAuthResponse> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/deployment/authorize_preview", self.provision_host);
        let resp = client
            .post(url)
            .bearer_auth(&self.access_token)
            .json(&args)
            .send()
            .await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!(anyhow::anyhow!(e).context(format!(
                "preview_deployment_credentials failed: {}",
                resp.text().await?
            )));
        }
        Ok(resp.json::<DeploymentAuthResponse>().await?)
    }

    pub async fn prod_deployment_credentials(
        &self,
        args: DeploymentAuthProdArgs,
    ) -> anyhow::Result<DeploymentAuthResponse> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/deployment/authorize_prod", self.provision_host);
        let resp = client
            .post(url)
            .bearer_auth(&self.access_token)
            .json(&args)
            .send()
            .await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!(anyhow::anyhow!(e).context(format!(
                "prod_deployment_credentials failed: {}",
                resp.text().await?
            )));
        }
        Ok(resp.json::<DeploymentAuthResponse>().await?)
    }
}
