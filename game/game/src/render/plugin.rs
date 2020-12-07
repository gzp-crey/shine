use crate::{
    app::AppError,
    render::{Context, FrameTarget, RenderError, Shader, Surface},
    World,
};
use serde::{Deserialize, Serialize};
use shine_ecs::resources::{Resource, ResourceWrite};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderConfig {
    pub swap_chain_format: wgpu::TextureFormat,
    pub enable_validation: bool,
    pub wgpu_trace: Option<String>,
}

impl World {
    pub fn render_plugin_name() -> &'static str {
        "render"
    }

    fn add_render_resource<T: Resource>(&mut self, resource: T) -> Result<(), AppError> {
        let _ = self
            .resources
            .insert(resource)
            .map_err(|err| AppError::plugin(Self::render_plugin_name(), err))?;
        Ok(())
    }

    /*fn get_render_resource<T: Resource>(&self) -> Result<ResourceRead<'_, T>, AppError> {
        Ok(self
            .resources
            .get::<T>()
            .map_err(|err| AppError::plugin_dependency(Self::render_plugin_name(), err))?)
    }*/

    fn get_mut_render_resource<T: Resource>(&self) -> Result<ResourceWrite<'_, T>, AppError> {
        Ok(self
            .resources
            .get_mut::<T>()
            .map_err(|err| AppError::plugin_dependency(Self::render_plugin_name(), err))?)
    }

    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), AppError> {
        let mut surface = self.get_mut_render_resource::<Surface>()?;
        let mut context = self.get_mut_render_resource::<Context>()?;
        let mut frame_output = self.get_mut_render_resource::<FrameTarget>()?;

        surface.set_size(size);
        let (output_texture, descriptor) = context.create_frame(&surface)?;
        frame_output.set(output_texture, descriptor);
        Ok(())
    }

    fn bake_resources(&mut self, gc: bool) {
        Shader::bake_resource(&mut self.resources, gc);
    }

    fn end_frame(&mut self) -> Result<(), AppError> {
        let mut context = self.get_mut_render_resource::<Context>()?;
        let mut frame_output = self.get_mut_render_resource::<FrameTarget>()?;
        context.submit_commands();
        frame_output.present();
        Ok(())
    }

    pub async fn add_render_plugin(
        &mut self,
        config: RenderConfig,
        wgpu_instance: wgpu::Instance,
        surface: Surface,
    ) -> Result<(), AppError> {
        log::info!("Adding render plugin");

        let context = Context::new(wgpu_instance, &surface, &config)
            .await
            .map_err(|err| RenderError::device_error("Failed to create context", err))?;

        let assetio = self.asset_io()?.clone();
        let device = context.device();

        self.add_render_resource(surface)?;
        self.add_render_resource(context)?;
        self.add_render_resource(FrameTarget::default())?;

        Shader::register_resource(&mut self.resources, assetio.clone(), device.clone());

        Ok(())
    }

    pub async fn remove_render_plugin(&mut self) -> Result<(), AppError> {
        //let _ = self.resources.remove::<RenderStores>();
        let _ = self.resources.remove::<FrameTarget>();
        let _ = self.resources.remove::<Context>();
        let _ = self.resources.remove::<Surface>();

        Shader::unregister_resource(&mut self.resources);
        Ok(())
    }

    pub fn render(&mut self, size: (u32, u32)) -> Result<(), AppError> {
        self.start_frame(size)?;
        self.bake_resources(false);
        let res = self.run_stage("render");
        self.end_frame()?;
        res
    }
}
