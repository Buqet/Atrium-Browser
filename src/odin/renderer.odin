package main

import "core:mem"
import "core:fmt"
import "core:os"
import "core:math"
import "vendor:glfw"
import "vendor:vulkan"

VK_API_VERSION :: vulkan.VK_API_VERSION_1_2
MAX_FRAMES_IN_FLIGHT :: 2
WINDOW_WIDTH :: 1280
WINDOW_HEIGHT :: 800

UI_Element_Type :: enum u8 {
    None,
    Button,
    Text,
    Input,
    Rectangle,
}

UI_Element :: struct {
    element_type: UI_Element_Type,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    color_a: f32,
    text: [128]byte,
    hovered: bool,
    clicked: bool,
}

Render_Command_Type :: enum u8 {
    None,
    Clear,
    Draw_Rectangle,
    Draw_Text,
    Draw_Image,
    Composite,
    Update_UI,
}

Render_Command :: struct {
    command: Render_Command_Type,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    color_a: f32,
    texture_id: u32,
    flags: u32,
}

Ring_Buffer :: struct {
    data: [256]Render_Command,
    head: u64,
    tail: u64,
    size: u64,
}

Vulkan_State :: struct {
    instance: vulkan.Instance,
    surface: vulkan.SurfaceKHR,
    physical_device: vulkan.PhysicalDevice,
    device: vulkan.Device,
    swapchain: vulkan.SwapchainKHR,
    swapchain_images: [MAX_FRAMES_IN_FLIGHT]vulkan.Image,
    swapchain_image_count: u32,
    swapchain_image_views: [MAX_FRAMES_IN_FLIGHT]vulkan.ImageView,
    render_pass: vulkan.RenderPass,
    graphics_pipeline: vulkan.Pipeline,
    pipeline_layout: vulkan.PipelineLayout,
    command_pool: vulkan.CommandPool,
    command_buffers: [MAX_FRAMES_IN_FLIGHT]vulkan.CommandBuffer,
    image_available_semaphores: [MAX_FRAMES_IN_FLIGHT]vulkan.Semaphore,
    render_finished_semaphores: [MAX_FRAMES_IN_FLIGHT]vulkan.Semaphore,
    fences: [MAX_FRAMES_IN_FLIGHT]vulkan.Fence,
}

window: glfw.Window = nil
vulkan_state: Vulkan_State = {}
ring_buffer: Ring_Buffer = {}
framebuffer_width: i32 = WINDOW_WIDTH
framebuffer_height: i32 = WINDOW_HEIGHT
ui_elements: [32]UI_Element = {}
ui_element_count: int = 0
current_frame: u32 = 0
framebuffer_resized: bool = false

@(export = "init_renderer")
init_renderer :: proc() -> bool {
    fmt.printf("[Odin] Initializing renderer...\n")
    
    if ~glfw.Init() {
        fmt.printf("[Odin] Failed to initialize GLFW\n")
        return false
    }

    glfw.Window_Hint(glfw.CLIENT_API, glfw.NO_API)
    glfw.Window_Hint(glfw.RESIZABLE, glfw.TRUE)
    glfw.Window_Hint(glfw.VISIBLE, glfw.TRUE)

    window = glfw.Create_Window(WINDOW_WIDTH, WINDOW_HEIGHT, "Atrium Browser", nil, nil)
    if window == nil {
        fmt.printf("[Odin] Failed to create GLFW window\n")
        glfw.Terminate()
        return false
    }

    glfw.Set_Window_User_Pointer(window, cast(rawptr)(window))
    glfw.Set_Framebuffer_Size_Callback(window, framebuffer_size_callback)

    fmt.printf("[Odin] Window created: %dx%d\n", WINDOW_WIDTH, WINDOW_HEIGHT)

    if ~init_vulkan() {
        fmt.printf("[Odin] Failed to initialize Vulkan\n")
        return false
    }

    init_ui()

    fmt.printf("[Odin] Renderer initialized successfully!\n")
    return true
}

@(export = "shutdown_renderer")
shutdown_renderer :: proc() {
    fmt.printf("[Odin] Shutting down renderer...\n")
    
    vulkan.Device_Wait_Idle(vulkan_state.device)
    cleanup_vulkan()
    
    if window != nil {
        glfw.Destroy_Window(window)
        window = nil
    }
    
    glfw.Terminate()
    fmt.printf("[Odin] Renderer shut down\n")
}

@(export = "push_render_command")
push_render_command :: proc(cmd: Render_Command) -> bool {
    if ring_buffer.size >= len(ring_buffer.data) {
        return false
    }
    
    ring_buffer.data[ring_buffer.tail] = cmd
    ring_buffer.tail = (ring_buffer.tail + 1) % len(ring_buffer.data)
    ring_buffer.size += 1
    
    return true
}

@(export = "process_render_commands")
process_render_commands :: proc() {
    for ring_buffer.size > 0 {
        cmd := ring_buffer.data[ring_buffer.head]
        ring_buffer.head = (ring_buffer.head + 1) % len(ring_buffer.data)
        ring_buffer.size -= 1
        
        execute_command(cmd)
    }
}

@(export = "render_frame")
render_frame :: proc() -> bool {
    if glfw.Window_Should_Close(window) {
        return false
    }

    glfw.Poll_Events()

    if framebuffer_resized {
        vulkan.Device_Wait_Idle(vulkan_state.device)
        cleanup_swapchain()
        create_swapchain()
        framebuffer_resized = false
    }

    process_render_commands()

    if ~draw_frame() {
        return false
    }

    current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT

    return true
}

@(export = "get_window_size")
get_window_size :: proc() -> (i32, i32) {
    return framebuffer_width, framebuffer_height
}

init_ui :: proc() {
    ui_element_count = 0

    add_ui_element(UI_Element{
        element_type = .Input,
        x = 10,
        y = 10,
        width = f32(WINDOW_WIDTH) - 120,
        height = 30,
        color_r = 0.2,
        color_g = 0.2,
        color_b = 0.25,
        color_a = 1.0,
    })

    add_ui_element(UI_Element{
        element_type = .Button,
        x = f32(WINDOW_WIDTH) - 100,
        y = 10,
        width = 90,
        height = 30,
        color_r = 0.2,
        color_g = 0.4,
        color_b = 0.8,
        color_a = 1.0,
        text = "Go",
    })
  
    add_ui_element(UI_Element{
        element_type = .Rectangle,
        x = 10,
        y = 50,
        width = f32(WINDOW_WIDTH) - 20,
        height = f32(WINDOW_HEIGHT) - 60,
        color_r = 0.15,
        color_g = 0.15,
        color_b = 0.18,
        color_a = 1.0,
    })
    
    fmt.printf("[Odin] UI initialized with %d elements\n", ui_element_count)
}

add_ui_element :: proc(elem: UI_Element) {
    if ui_element_count < len(ui_elements) {
        ui_elements[ui_element_count] = elem
        ui_element_count += 1
    }
}


init_vulkan :: proc() -> bool {
    fmt.printf("[Odin] Initializing Vulkan...\n")
    
    app_info: vulkan.Application_Info = {
        application_name = "Atrium Browser",
        application_version = vulkan.VK_MAKE_VERSION(1, 0, 0),
        engine_name = "Atrium Engine",
        engine_version = vulkan.VK_MAKE_VERSION(1, 0, 0),
        api_version = VK_API_VERSION,
    }
    
    instance_info: vulkan.Instance_Create_Info = {
        application_info = &app_info,
    }
    
    result := vulkan.Create_Instance(&instance_info, nil, &vulkan_state.instance)
    if result != vulkan.VK_SUCCESS {
        fmt.printf("[Odin] Failed to create Vulkan instance: %d\n", result)
        return false
    }
    fmt.printf("[Odin] Vulkan instance created\n")

    result = glfw.Create_Window_Surface(vulkan_state.instance, window, &vulkan_state.surface)
    if result != vulkan.VK_SUCCESS {
        fmt.printf("[Odin] Failed to create window surface\n")
        return false
    }
    fmt.printf("[Odin] Surface created\n")

    if ~select_physical_device() {
        return false
    }

    if ~create_logical_device() {
        return false
    }

    if ~create_swapchain() {
        return false
    }

    if ~create_render_pass() {
        return false
    }

    if ~create_command_pool() {
        return false
    }

    if ~create_sync_objects() {
        return false
    }

    fmt.printf("[Odin] Vulkan initialized successfully!\n")
    return true
}

select_physical_device :: proc() -> bool {
    device_count: u32 = 0
    vulkan.Enumerate_Physical_Devices(vulkan_state.instance, &device_count, nil)
    
    if device_count == 0 {
        fmt.printf("[Odin] No Vulkan physical devices found\n")
        return false
    }
    
    devices := make([]vulkan.Physical_Device, device_count)
    vulkan.Enumerate_Physical_Devices(vulkan_state.instance, &device_count, &devices[0])
    
    vulkan_state.physical_device = devices[0]
    
    props: vulkan.Physical_Device_Properties
    vulkan.Get_Physical_Device_Properties(vulkan_state.physical_device, &props)
    fmt.printf("[Odin] Selected GPU: %.*s\n", len(props.device_name[:]), props.device_name[:])
    
    return true
}

create_logical_device :: proc() -> bool {
    queue_info: vulkan.Device_Queue_Create_Info = {
        queue_count = 1,
    }
    
    device_info: vulkan.Device_Create_Info = {
        queue_create_info_count = 1,
        queue_create_infos = &queue_info,
    }
    
    result := vulkan.Create_Device(
        vulkan_state.physical_device,
        &device_info,
        nil,
        &vulkan_state.device,
    )
    
    return result == vulkan.VK_SUCCESS
}

create_swapchain :: proc() -> bool {
    capabilities: vulkan.Surface_Capabilities_KHR
    vulkan.Get_Physical_Device_Surface_Capabilities_KHR(
        vulkan_state.physical_device,
        vulkan_state.surface,
        &capabilities,
    )
    
    framebuffer_width = i32(capabilities.current_extent.width)
    framebuffer_height = i32(capabilities.current_extent.height)
    
    swapchain_info: vulkan.Swapchain_Create_Info_KHR = {
        surface = vulkan_state.surface,
        min_image_count = 2,
        image_format = capabilities.surface_formats[0].format,
        image_color_space = capabilities.surface_formats[0].color_space,
        image_extent = capabilities.current_extent,
        image_array_layers = 1,
        image_usage = vulkan.VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
        pre_transform = capabilities.current_transform,
        composite_alpha = vulkan.VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
        present_mode = vulkan.VK_PRESENT_MODE_FIFO_KHR,
        clipped = vulkan.VK_TRUE,
    }
    
    result := vulkan.Create_Swapchain_KHR(
        vulkan_state.device,
        &swapchain_info,
        nil,
        &vulkan_state.swapchain,
    )
    
    if result != vulkan.VK_SUCCESS {
        return false
    }
    
    vulkan.Get_Swapchain_Images_KHR(
        vulkan_state.device,
        vulkan_state.swapchain,
        &vulkan_state.swapchain_image_count,
        nil,
    )
    
    vulkan.Get_Swapchain_Images_KHR(
        vulkan_state.device,
        vulkan_state.swapchain,
        &vulkan_state.swapchain_image_count,
        &vulkan_state.swapchain_images[0],
    )
    
    for i in 0..<vulkan_state.swapchain_image_count {
        view_info: vulkan.ImageView_Create_Info = {
            image = vulkan_state.swapchain_images[i],
            view_type = vulkan.VK_IMAGE_VIEW_TYPE_2D,
            format = swapchain_info.image_format,
            subresource_range = {
                aspect_mask = vulkan.VK_IMAGE_ASPECT_COLOR_BIT,
                base_mip_level = 0,
                level_count = 1,
                base_array_layer = 0,
                layer_count = 1,
            },
        }
        
        result = vulkan.Create_ImageView(
            vulkan_state.device,
            &view_info,
            nil,
            &vulkan_state.swapchain_image_views[i],
        )
        
        if result != vulkan.VK_SUCCESS {
            return false
        }
    }
    
    return true
}

create_render_pass :: proc() -> bool {
    color_attachment: vulkan.Attachment_Description = {
        format = vulkan.VK_FORMAT_B8G8R8A8_UNORM,
        samples = vulkan.VK_SAMPLE_COUNT_1_BIT,
        load_op = vulkan.VK_ATTACHMENT_LOAD_OP_CLEAR,
        store_op = vulkan.VK_ATTACHMENT_STORE_OP_STORE,
        initial_layout = vulkan.VK_IMAGE_LAYOUT_UNDEFINED,
        final_layout = vulkan.VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
    }
    
    subpass: vulkan.Subpass_Description = {
        pipeline_bind_point = vulkan.VK_PIPELINE_BIND_POINT_GRAPHICS,
        color_attachment_count = 1,
    }
    
    render_pass_info: vulkan.Render_Pass_Create_Info = {
        attachment_count = 1,
        attachments = &color_attachment,
        subpass_count = 1,
        subpasses = &subpass,
    }
    
    result := vulkan.Create_Render_Pass(
        vulkan_state.device,
        &render_pass_info,
        nil,
        &vulkan_state.render_pass,
    )
    
    return result == vulkan.VK_SUCCESS
}

create_command_pool :: proc() -> bool {
    pool_info: vulkan.Command_Pool_Create_Info = {
        flags = vulkan.VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
    }
    
    result := vulkan.Create_Command_Pool(
        vulkan_state.device,
        &pool_info,
        nil,
        &vulkan_state.command_pool,
    )
    
    return result == vulkan.VK_SUCCESS
}

create_sync_objects :: proc() -> bool {
    semaphore_info: vulkan.Semaphore_Create_Info = {}
    fence_info: vulkan.Fence_Create_Info = {
        flags = vulkan.VK_FENCE_CREATE_SIGNALED_BIT,
    }
    
    for i in 0..<MAX_FRAMES_IN_FLIGHT {
        if vulkan.Create_Semaphore(vulkan_state.device, &semaphore_info, nil, 
            &vulkan_state.image_available_semaphores[i]) != vulkan.VK_SUCCESS {
            return false
        }
        
        if vulkan.Create_Semaphore(vulkan_state.device, &semaphore_info, nil,
            &vulkan_state.render_finished_semaphores[i]) != vulkan.VK_SUCCESS {
            return false
        }
        
        if vulkan.Create_Fence(vulkan_state.device, &fence_info, nil,
            &vulkan_state.fences[i]) != vulkan.VK_SUCCESS {
            return false
        }
    }
    
    return true
}

cleanup_vulkan :: proc() {
    for i in 0..<MAX_FRAMES_IN_FLIGHT {
        vulkan.Destroy_Semaphore(vulkan_state.device, vulkan_state.image_available_semaphores[i], nil)
        vulkan.Destroy_Semaphore(vulkan_state.device, vulkan_state.render_finished_semaphores[i], nil)
        vulkan.Destroy_Fence(vulkan_state.device, vulkan_state.fences[i], nil)
    }
    
    cleanup_swapchain()
    
    vulkan.Destroy_Command_Pool(vulkan_state.device, vulkan_state.command_pool, nil)
    vulkan.Destroy_Device(vulkan_state.device, nil)
    vulkan.Destroy_Surface_KHR(vulkan_state.instance, vulkan_state.surface, nil)
    vulkan.Destroy_Instance(vulkan_state.instance, nil)
}

cleanup_swapchain :: proc() {
    for i in 0..<vulkan_state.swapchain_image_count {
        vulkan.Destroy_ImageView(vulkan_state.device, vulkan_state.swapchain_image_views[i], nil)
    }
    vulkan.Destroy_Swapchain_KHR(vulkan_state.device, vulkan_state.swapchain, nil)
}




draw_frame :: proc() -> bool {
    vulkan.Wait_For_Fences(vulkan_state.device, 1, &vulkan_state.fences[current_frame], vulkan.VK_TRUE, u64.max)
    
    image_index: u32 = 0
    result := vulkan.Acquire_Next_Image_KHR(
        vulkan_state.device,
        vulkan_state.swapchain,
        u64.max,
        vulkan_state.image_available_semaphores[current_frame],
        vulkan.VK_NULL_HANDLE,
        &image_index,
    )
    
    if result == vulkan.VK_ERROR_OUT_OF_DATE_KHR {
        return true
    }
    
    clear_values: [1]vulkan.Clear_Value = {{
        color = {float32 = [4]f32{0.1, 0.1, 0.12, 1.0}},
    }}
    
    render_pass_info: vulkan.Render_Pass_Begin_Info = {
        render_pass = vulkan_state.render_pass,
        framebuffer = vulkan_state.swapchain_image_views[image_index],
        render_area = {
            offset = {0, 0},
            extent = {
                width = u32(framebuffer_width),
                height = u32(framebuffer_height),
            },
        },
        clear_value_count = 1,
        clear_values = &clear_values[0],
    }
    
    submit_info: vulkan.Submit_Info = {
        wait_semaphore_count = 1,
        wait_semaphores = &vulkan_state.image_available_semaphores[current_frame],
        wait_dst_stage_mask = &([1]vulkan.Pipeline_Stage_Flags{.COLOR_ATTACHMENT_OUTPUT}[0]),
        command_buffer_count = 1,
        command_buffers = &vulkan_state.command_buffers[current_frame],
        signal_semaphore_count = 1,
        signal_semaphores = &vulkan_state.render_finished_semaphores[current_frame],
    }
    
    vulkan.Reset_Fences(vulkan_state.device, 1, &vulkan_state.fences[current_frame])
    
    result = vulkan.Queue_Submit(
        vulkan.Device_Get_Queue(vulkan_state.device, 0, 0),
        1,
        &submit_info,
        vulkan_state.fences[current_frame],
    )
    
    if result != vulkan.VK_SUCCESS {
        return false
    }
    
    return true
}

execute_command :: proc(cmd: Render_Command) {
    switch cmd.command {
    case .Clear:
    case .Draw_Rectangle:
    case .Draw_Text:
    case .Draw_Image:
    case .Composite:
    case .Update_UI:
    case .None:
    }
}

framebuffer_size_callback :: proc(w: glfw.Window, width: int, height: int) {
    framebuffer_width = i32(width)
    framebuffer_height = i32(height)
    framebuffer_resized = true
    fmt.printf("[Odin] Window resized to %dx%d\n", width, height)
}

main :: proc() {
    if ~init_renderer() {
        os.exit(1)
    }
    
    defer shutdown_renderer()
    
    for {
        if ~render_frame() {
            break
        }
    }
}
