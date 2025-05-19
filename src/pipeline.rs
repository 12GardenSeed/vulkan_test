use crate::{RenderApp, AppData, Vertex, create_texture_image_view, create_image_view};
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, Instance};
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use anyhow::{ Result };
use vulkanalia::vk::{KhrSurfaceExtension, KhrSwapchainExtension, Pipeline};

#[derive(Debug)]
pub struct EnginePipeline {
    // render_server:
    pipeline: vk::Pipeline,
    pub vec_vertex:Vec<Vertex>,
    pub vec_index:Vec<u32>,
    // // TODO 不是pipeline内置的结构，放到更合理的位置
    // // pipeline当此运行绑定的纹理
    pub vec_image: Vec<vk::Image>,
    pub vec_image_view: Vec<vk::ImageView>,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout: vk::PipelineLayout,
}

impl <'a>EnginePipeline {
    pub fn new(app:&'a mut RenderApp) -> Result<(Self)> {
        let vert = include_bytes!("../assets/ShaderOut/vert.spv");
        let frag = include_bytes!("../assets/ShaderOut/frag.spv");

        let vert_module = unsafe {
            create_shader_module(&app.device, &vert[..])
        }?;
        let frag_module = unsafe {
            create_shader_module(&app.device, &frag[..])
        }?;
        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(b"main\0");
        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(b"main\0");
        let binding_descriptions = &[Vertex::binding_description()];
        let attribute_descriptions = Vertex::attribute_descriptions();
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(app.data.swapchain_extent.width as f32)
            .height(app.data.swapchain_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(app.data.swapchain_extent);
        let viewports = &[viewport];
        let scissors = &[scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .scissors(scissors);
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::_1);
        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);
        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([1.0, 0.0, 0.0, 0.0]);
        let descriptor_set_layout = unsafe{
            create_descriptor_set_layout(&app.device)
        }?;
        let set_layouts = &[descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(set_layouts);
        let pipeline_layout = unsafe {
            app.device.create_pipeline_layout(&layout_info, None)
        }?;

        let stages = &[vert_stage, frag_stage];
        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0) // Optional.
            .max_depth_bounds(1.0) // Optional.
            .stencil_test_enable(false);
        // .front(/* vk::StencilOpState */) // Optional.
        // .back(/* vk::StencilOpState */); // Optional.

        let info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .layout(pipeline_layout)
            .render_pass(app.data.render_pass)
            .subpass(0);
        let pipeline = unsafe {
            app.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
                .0[0]
        };
        let descriptor_pool = app.data.descriptor_pool;
        unsafe {
            &app.device.destroy_shader_module(vert_module, None);
            &app.device.destroy_shader_module(frag_module, None);
        }
        Ok(
            Self {
                pipeline,
                descriptor_pool,
                descriptor_set_layout,
                pipeline_layout,
                vec_index: vec![],
                vec_vertex: vec![],
                vec_image: vec![],
                vec_image_view: vec![],
            }
        )
    }
    pub fn update_texture_vec(&mut self, app: &RenderApp, vec_image: Vec<Vec<u8>>, width: u32, height: u32) -> Result<()> {
        self.vec_image.clear();
        self.vec_image_view.clear();
        vec_image
            .iter()
            .for_each(|vv| {
                self.vec_image.push(unsafe {
                    let (image, _) = app.create_texture_image(vv, width, height).unwrap();
                    image
                })
            });
        self.vec_image
            .iter()
            .for_each(|image| {
                self.vec_image_view.push(unsafe {
                    create_image_view(
                        &app.device,
                        *image,
                        vk::Format::R8G8B8A8_SRGB,
                        vk::ImageAspectFlags::COLOR,
                    ).unwrap()
                })
            });
        Ok(())
    }
    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
}


unsafe fn create_descriptor_set_layout(device: &vulkanalia::Device) -> anyhow::Result<(vk::DescriptorSetLayout)> {
    let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = &[ubo_binding, sampler_binding];

    let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

    Ok(device.create_descriptor_set_layout(&info, None)?)
}

pub unsafe fn create_shader_module(device: &vulkanalia::Device, bytecode: &[u8]) -> anyhow::Result<vk::ShaderModule> {
    let b_code = Bytecode::new(bytecode).unwrap();
    let info = vk::ShaderModuleCreateInfo::builder()
        .code(b_code.code())
        .code_size(bytecode.len());
    Ok(device.create_shader_module(&info, None)?)
}