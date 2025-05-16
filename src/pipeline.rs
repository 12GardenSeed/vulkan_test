use crate::{AppData, Vertex};
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, Instance};
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use anyhow::{ Result };
use vulkanalia::vk::{KhrSurfaceExtension, KhrSwapchainExtension, Pipeline};

#[derive(Clone, Debug)]
struct EnginePipeline {
    pipeline: vk::Pipeline,
    vec_vertex:Vec<Vertex>,
    vec_index:Vec<u32>,
    // // TODO 不是pipeline内置的结构，放到更合理的位置
    // // pipeline当此运行绑定的纹理
    vec_image: Vec<vk::Image>,
    descriptor_pool: vk::DescriptorPool,
}

impl EnginePipeline {
    pub fn new(device: &Device, descriptor_pool: vk::DescriptorPool, instance: Instance, data: &mut AppData) -> Result<(Self)> {
        let vert = include_bytes!("../assets/ShaderOut/vert.spv");
        let frag = include_bytes!("../assets/ShaderOut/frag.spv");

        let vert_module = unsafe {
            create_shader_module(device, &vert[..])
        }?;
        let frag_module = unsafe {
            create_shader_module(device, &frag[..])
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
            .width(data.swapchain_extent.width as f32)
            .height(data.swapchain_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(data.swapchain_extent);
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

        let set_layouts = &[data.descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(set_layouts);
        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&layout_info, None)
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
            .layout(data.pipeline_layout)
            .render_pass(data.render_pass)
            .subpass(0);
        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
                .0[0]
        };
        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }
        Ok(
            Self {
                pipeline,
                descriptor_pool,
                vec_index: vec![],
                vec_vertex: vec![],
                vec_image: vec![],
            }
        )
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