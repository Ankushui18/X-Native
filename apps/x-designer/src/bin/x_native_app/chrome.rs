use std::{num::NonZeroUsize, sync::Arc};
use vello::{kurbo::{Affine, BezPath, Ellipse, Point, Rect, RoundedRect}, peniko::Color, AaConfig, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::{Backends, SurfaceConfiguration};
use winit::{
    event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, ModifiersState, NamedKey},
    window::Window,
};
use x_native::text::{encode_rich_text, Align, FontManager, Span, SystemFonts, TextBlockStyle};

#[derive(Clone, Copy)]
struct Palette {
    bg: Color, surface: Color, surface2: Color, border: Color,
    text: Color, muted: Color, faint: Color, accent: Color, accent_soft: Color,
    canvas: Color, white: Color, field: Color,
}
impl Default for Palette {
    // Built FROM theme.rs — the only place chrome color is defined.
    fn default() -> Self { Self {
        bg: crate::theme::C_BASE, surface: crate::theme::C_PANEL,
        surface2: crate::theme::C_RAISED, border: crate::theme::C_EDGE,
        text: crate::theme::C_TEXT, muted: crate::theme::C_DIM,
        faint: crate::theme::C_FAINT, accent: crate::theme::C_ACCENT,
        accent_soft: crate::theme::C_ACCENT_MUTED,
        canvas: crate::theme::C_CANVAS, white: Color::WHITE,
        field: crate::theme::C_FIELD,
    }}
}

pub struct XNativeApp {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    renderer: Renderer,
    scene: Scene,
    fonts: FontManager,
    font: usize,
    palette: Palette,
    mouse: (f64,f64),
    selected: bool,
    zoom: f64,
    active_tool: usize,
    command_open: bool,
    modifiers: ModifiersState,
}

impl XNativeApp {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: Backends::all(), ..Default::default() });
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() }).await.expect("adapter");
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.expect("device");
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = SurfaceConfiguration { usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: size.width.max(1), height: size.height.max(1), present_mode: caps.present_modes.iter().copied().find(|m| *m == wgpu::PresentMode::Fifo).unwrap_or(caps.present_modes[0]), desired_maximum_frame_latency:2, alpha_mode: caps.alpha_modes[0], view_formats: vec![] };
        surface.configure(&device, &config);
        let renderer = Renderer::new(&device, RendererOptions { use_cpu:false, antialiasing_support:vello::AaSupport::all(), num_init_threads:NonZeroUsize::new(1), ..Default::default() }).expect("renderer");
        let mut fonts = FontManager::new();
        fonts.load_system_fonts();
        let font = SystemFonts::enumerate().load_into(&mut fonts,"Inter","Regular").ok()
            .or_else(|| fonts.font_index("Inter Regular"))
            .or_else(|| fonts.font_index("Inter"))
            .or_else(|| fonts.default_font()).unwrap_or(0);
        Self { window, surface, device, queue, config, renderer, scene:Scene::new(), fonts, font, palette:Palette::default(), mouse:(0.0,0.0), selected:false, zoom:1.0, active_tool:0, command_open:false, modifiers:ModifiersState::default() }
    }

    pub fn handle_event(&mut self, event: Event<()>, elwt: &ActiveEventLoop) {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => { self.resize(size.width, size.height); }
                WindowEvent::CursorMoved { position, .. } => { self.mouse=(position.x,position.y); self.window.request_redraw(); }
                WindowEvent::MouseInput { state:ElementState::Pressed, button:MouseButton::Left, .. } => { self.on_click(); self.window.request_redraw(); }
                WindowEvent::MouseWheel { delta, .. } => { let d=match delta { winit::event::MouseScrollDelta::LineDelta(_,y)=>y as f64, winit::event::MouseScrollDelta::PixelDelta(p)=>p.y/80.0 }; let factor=if d>0.0{1.12}else{0.89}; self.zoom=(self.zoom*factor).clamp(0.05,16.0); self.window.request_redraw(); }
                WindowEvent::ModifiersChanged(m) => self.modifiers=m.state(),
                WindowEvent::KeyboardInput { event:KeyEvent { logical_key, state:ElementState::Pressed, .. }, .. } => { self.on_key(logical_key); self.window.request_redraw(); }
                WindowEvent::RedrawRequested => { self.render(); }
                _ => {}
            },
            Event::AboutToWait => self.window.request_redraw(),
            _ => {}
        }
    }

    fn resize(&mut self,w:u32,h:u32){ self.config.width=w.max(1); self.config.height=h.max(1); self.surface.configure(&self.device,&self.config); }
    fn top_h(&self)->f64 { 48.0 }
    fn left_w(&self)->f64 { if self.config.width < 1180 { 208.0 } else { 232.0 } }
    fn right_w(&self)->f64 { if self.config.width < 1180 { 272.0 } else { 296.0 } }
    fn shortcut(&self)->&'static str { if cfg!(target_os="macos") { "⌘ K" } else { "Ctrl K" } }
    fn hit_canvas_object(&self,x:f64,y:f64)->bool { let left=self.left_w(); let top=self.top_h(); let right=self.right_w(); let cw=self.config.width as f64-left-right; let ch=self.config.height as f64-top-24.0; let s=(cw/790.0).min(ch/730.0).min(1.0)*self.zoom; let aw=710.0*s; let ah=690.0*s; let ax=left+(cw-aw)*0.5; let ay=top+(ch-ah)*0.5; x>=ax && x<=ax+aw && y>=ay && y<=ay+ah }

    fn on_click(&mut self){
        let (x,y)=self.mouse;
        if self.command_open { self.command_open=false; return; }
        let tools_x=(self.config.width as f64*0.5-234.0).max(self.left_w()+24.0);
        let ty=self.top_h()+16.0;
        if y>=ty && y<ty+44.0 && x>=tools_x && x<tools_x+432.0 {
            self.active_tool=((x-tools_x)/48.0).floor().clamp(0.0,8.0) as usize; return;
        }
        self.selected=self.hit_canvas_object(x,y);
    }

    fn on_key(&mut self,key:Key){
        let command=self.modifiers.super_key() || self.modifiers.control_key();
        match key {
            Key::Character(c) if command && c.eq_ignore_ascii_case("k") => self.command_open=!self.command_open,
            Key::Named(NamedKey::Escape) => self.command_open=false,
            Key::Character(c) if c.eq_ignore_ascii_case("v") => self.active_tool=0,
            Key::Character(c) if c.eq_ignore_ascii_case("f") => self.active_tool=1,
            Key::Character(c) if c.eq_ignore_ascii_case("t") => self.active_tool=2,
            Key::Character(c) if c.eq_ignore_ascii_case("r") => self.active_tool=3,
            Key::Character(c) if c.eq_ignore_ascii_case("o") => self.active_tool=4,
            Key::Character(c) if c.eq_ignore_ascii_case("p") => self.active_tool=5,
            Key::Character(c) if c.eq_ignore_ascii_case("i") => self.active_tool=7,
            Key::Character(c) if c.eq_ignore_ascii_case("h") => self.active_tool=8,
            _ => {}
        }
    }

    fn render(&mut self) {
        self.scene.reset();
        let w=self.config.width as f64; let h=self.config.height as f64; let p=self.palette;
        let left=self.left_w(); let right=self.right_w(); let top=self.top_h();
        self.rect(0.0,0.0,w,h,p.bg,0.0);
        // Canvas first; chrome sits above it.
        self.rect(left,top,w-left-right,h-top-24.0,p.canvas,0.0);
        if self.zoom>=8.0 { self.pixel_grid(left,top,w-left-right,h-top-24.0); }
        self.neutral_art(left,top,w-left-right,h-top-24.0,p);
        self.left_panel(p); self.right_panel(p); self.top_bar(p); self.floating_toolbar(p); self.bottom_bar(p);
        if self.command_open { self.command_palette(p); }

        let output=self.surface.get_current_texture();
        let frame=match output { Ok(f)=>f, Err(_)=>{self.surface.configure(&self.device,&self.config); return;} };
        let view=frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let params=RenderParams { base_color:p.bg, width:self.config.width, height:self.config.height, antialiasing_method:AaConfig::Area };
        if self.renderer.render_to_texture(&self.device,&self.queue,&self.scene,&view,&params).is_ok(){ frame.present(); }
    }

    fn top_bar(&mut self,p:Palette){
        let w=self.config.width as f64;
        let top=self.top_h();
        self.rect(0.0,0.0,w,top,p.surface,0.0); self.line(0.0,top-0.5,w,top-0.5,p.border,1.0);
        self.round(10.0,8.0,32.0,32.0,p.accent,8.0); self.draw_text("X",21.0,15.0,13.0,crate::theme::C_ON_ACCENT);
        self.draw_text("Untitled",54.0,10.0,12.0,p.text); self.draw_text("Saved locally",54.0,27.0,8.5,p.faint);
        if w>1120.0 { let sx=w-252.0; self.round(sx,8.0,116.0,32.0,p.field,8.0); self.search_icon(sx+12.0,19.0,p.muted); self.draw_text("Quick actions",sx+34.0,17.0,9.5,p.muted); }
        self.round(w-126.0,8.0,116.0,32.0,p.accent,8.0); self.draw_text("Share",w-80.0,17.0,11.0,crate::theme::C_ON_ACCENT);
    }
    /// Framer-style tool dock: detached from the header, floating over the
    /// canvas with its own shadow — not embedded in the top strip like the
    /// old Figma-only layout.
    fn floating_toolbar(&mut self,p:Palette){
        let w=self.config.width as f64;
        let tools_x=(w*0.5-234.0).max(self.left_w()+24.0);
        let ty=self.top_h()+16.0;
        self.round(tools_x-8.0,ty+6.0,448.0,48.0,Color::from_rgba8(0,0,0,90),14.0);
        self.round(tools_x-4.0,ty,440.0,44.0,p.surface2,12.0);
        self.stroke_round(tools_x-4.0,ty,440.0,44.0,p.border,1.0,12.0);
        let (mx,my)=self.mouse;
        let hovered = if my>=ty && my<ty+44.0 && mx>=tools_x && mx<tools_x+432.0 {
            Some(((mx-tools_x)/48.0).floor().clamp(0.0,8.0) as usize)
        } else { None };
        for (i,shortcut) in ["V","F","S","R","O","P","T","I","H"].iter().enumerate(){ self.center_tool(tools_x+i as f64*48.0,ty,i,shortcut,i==self.active_tool,hovered==Some(i),p); }
    }
    fn left_panel(&mut self,p:Palette){
        let top=self.top_h(); let width=self.left_w();
        let (mx,my)=self.mouse;
        let hover_row=|ry:f64|->bool{ mx>=8.0 && mx<width-8.0 && my>=ry-5.0 && my<ry+21.0 };
        self.rect(0.0,top,width,self.config.height as f64-top-24.0,p.surface,0.0); self.line(width-0.5,top,width-0.5,self.config.height as f64-24.0,p.border,1.0);
        self.draw_text("Layers",14.0,top+13.0,11.5,p.text); self.draw_text("Assets",62.0,top+13.0,11.5,p.faint); self.draw_text("Components",106.0,top+13.0,11.5,p.faint);
        self.line(12.0,top+33.0,width-12.0,top+33.0,p.border,1.0);
        self.round(12.0,top+44.0,width-24.0,30.0,p.field,6.0); self.search_icon(22.0,top+53.0,p.faint); self.draw_text("Search layers",40.0,top+52.0,10.5,p.faint);
        self.draw_text("PAGE",14.0,top+92.0,9.0,p.faint);
        self.round(8.0,top+108.0,width-16.0,28.0,p.accent_soft,5.0); self.draw_text("Page 1",26.0,top+116.0,10.5,p.text);
        self.draw_text("LAYERS",14.0,top+154.0,9.5,p.faint);
        self.layer_row(8.0,top+176.0,"Desktop",p,false,hover_row(top+176.0)); self.layer_row(24.0,top+204.0,"Hero section",p,self.selected,hover_row(top+204.0)); self.layer_row(40.0,top+232.0,"Navigation",p,false,hover_row(top+232.0)); self.layer_row(40.0,top+260.0,"Heading",p,false,hover_row(top+260.0)); self.layer_row(40.0,top+288.0,"Button",p,false,hover_row(top+288.0)); self.layer_row(40.0,top+316.0,"Media",p,false,hover_row(top+316.0));
        let by=self.config.height as f64-112.0; self.line(12.0,by-10.0,width-12.0,by-10.0,p.border,1.0); self.draw_text("Libraries",14.0,by,11.0,p.muted); self.draw_text("Variables",14.0,by+28.0,11.0,p.muted); self.draw_text("Plugins",14.0,by+56.0,11.0,p.muted);
    }
    fn right_panel(&mut self,p:Palette){
        let width=self.right_w(); let x=self.config.width as f64-width; let top=self.top_h(); let h=self.config.height as f64-top-24.0;
        self.rect(x,top,width,h,p.surface,0.0); self.line(x+0.5,top,x+0.5,h+top,p.border,1.0);
        self.draw_text("Design",x+16.0,top+13.0,11.5,p.text); self.draw_text("Prototype",x+76.0,top+13.0,11.5,p.faint); self.draw_text("Code",x+148.0,top+13.0,11.5,p.faint); self.line(x+14.0,top+33.0,x+57.0,top+33.0,p.accent,2.0);
        if self.selected { self.inspector_selected(x,p); } else { self.inspector_empty(x,p); }
    }
    fn inspector_empty(&mut self,x:f64,p:Palette){ let top=self.top_h(); let width=self.right_w(); self.draw_text("Page",x+16.0,top+54.0,13.0,p.text); self.section(x,top+80.0,"CANVAS",p); self.prop(x,top+112.0,"Zoom",format!("{}%",(self.zoom*100.0) as i32),p); self.prop(x,top+146.0,"Background","#12151C".into(),p); self.round(x+16.0,top+202.0,width-32.0,86.0,p.surface2,8.0); self.draw_text("Nothing selected",x+30.0,top+220.0,12.0,p.text); self.draw_text("Choose a layer or click the canvas",x+30.0,top+244.0,10.0,p.muted); self.draw_text("to edit its properties.",x+30.0,top+261.0,10.0,p.muted); }
    fn inspector_selected(&mut self,x:f64,p:Palette){ let top=self.top_h(); let width=self.right_w(); self.draw_text("Card",x+16.0,top+52.0,14.0,p.text); self.draw_text("Frame",x+16.0,top+72.0,10.0,p.faint); self.section(x,top+98.0,"ALIGN",p); for i in 0..6 { let bx=x+16.0+i as f64*42.0; self.round(bx,top+126.0,34.0,28.0,p.field,5.0); self.align_icon(bx+9.0,top+133.0,i,p.muted); } self.section(x,top+170.0,"POSITION & SIZE",p); self.prop_pair(x,top+202.0,"X","420","Y","230",p); self.prop_pair(x,top+236.0,"W","320","H","180",p); self.section(x,top+282.0,"AUTO LAYOUT",p); self.round(x+16.0,top+310.0,width-32.0,34.0,p.surface2,6.0); self.draw_text("Horizontal   16 gap   24 padding",x+28.0,top+320.0,10.0,p.text); self.section(x,top+362.0,"APPEARANCE",p); self.prop(x,top+394.0,"Fill","#FFFFFF".into(),p); self.prop(x,top+428.0,"Stroke","None".into(),p); self.prop_pair(x,top+462.0,"Radius","18","Opacity","100%",p); self.section(x,top+508.0,"EFFECTS",p); self.round(x+16.0,top+536.0,width-32.0,34.0,p.surface2,6.0); self.draw_text("Drop shadow",x+28.0,top+546.0,10.5,p.muted); self.draw_text("+",x+width-34.0,top+544.0,14.0,p.muted); self.section(x,top+588.0,"EXPORT",p); self.round(x+16.0,top+616.0,width-32.0,34.0,p.surface2,6.0); self.draw_text("+  Add export setting",x+28.0,top+626.0,10.5,p.muted); }
    fn bottom_bar(&mut self,p:Palette){ let y=self.config.height as f64-24.0; let w=self.config.width as f64; self.rect(0.0,y,w,24.0,p.surface,0.0); self.line(0.0,y,w,y,p.border,1.0); self.draw_text("Page 1",12.0,y+6.0,9.5,p.faint); self.draw_text(if cfg!(target_os="macos") { "macOS native" } else if cfg!(target_os="windows") { "Windows native" } else { "Desktop native" },70.0,y+6.0,9.5,p.faint); self.draw_text("?",w-190.0,y+5.0,10.0,p.muted); self.draw_text("−",w-154.0,y+4.0,13.0,p.muted); self.draw_text(&format!("{}%",(self.zoom*100.0) as i32),w-126.0,y+6.0,9.5,p.text); self.draw_text("+",w-76.0,y+4.0,13.0,p.muted); self.draw_text("Fit",w-45.0,y+6.0,9.5,p.muted); }
    fn page_art(&mut self,x:f64,y:f64,w:f64,h:f64,p:Palette){ let s=(w/790.0).min(h/730.0).min(1.0)*self.zoom; let aw=710.0*s; let ah=690.0*s; let ax=x+(w-aw)*0.5; let ay=y+(h-ah)*0.5; self.round(ax-6.0,ay+10.0,aw+12.0,ah+10.0,Color::from_rgba8(0,0,0,88),8.0); self.round(ax,ay,aw,ah,Color::from_rgb8(0xf5,0xf7,0xfa),4.0); self.rect(ax,ay,132.0*s,ah,Color::from_rgb8(0x11,0x18,0x27),0.0); self.round(ax+18.0*s,ay+20.0*s,24.0*s,24.0*s,p.accent,12.0*s); self.draw_text("NOVA",ax+52.0*s,ay+24.0*s,10.0*s,Color::WHITE); for (i,name) in ["Overview","Analytics","Transactions","Customers","Reports"].iter().enumerate(){ if i==0 { self.round(ax+14.0*s,ay+(69.0+i as f64*34.0)*s,104.0*s,34.0*s,p.accent_soft,7.0*s); } self.draw_text(name,ax+26.0*s,ay+(80.0+i as f64*34.0)*s,8.0*s,Color::from_rgb8(0x94,0xa3,0xb8)); } self.rect(ax+132.0*s,ay,578.0*s,58.0*s,Color::WHITE,0.0); self.draw_text("Overview",ax+156.0*s,ay+15.0*s,11.0*s,Color::from_rgb8(0x0f,0x17,0x2a)); self.draw_text("Good morning, Akshay",ax+156.0*s,ay+82.0*s,16.0*s,Color::from_rgb8(0x0f,0x17,0x2a)); for i in 0..3 { self.round(ax+(157.0+i as f64*176.0)*s,ay+140.0*s,162.0*s,105.0*s,Color::WHITE,10.0*s); } self.draw_text("TOTAL BALANCE",ax+173.0*s,ay+158.0*s,7.0*s,Color::from_rgb8(0x64,0x74,0x8b)); self.draw_text("$84,240.50",ax+173.0*s,ay+182.0*s,17.0*s,Color::from_rgb8(0x0f,0x17,0x2a)); self.draw_text("REVENUE",ax+349.0*s,ay+158.0*s,7.0*s,Color::from_rgb8(0x64,0x74,0x8b)); self.draw_text("$42,840",ax+349.0*s,ay+182.0*s,17.0*s,Color::from_rgb8(0x0f,0x17,0x2a)); self.draw_text("EXPENSES",ax+525.0*s,ay+158.0*s,7.0*s,Color::from_rgb8(0x64,0x74,0x8b)); self.draw_text("$18,620",ax+525.0*s,ay+182.0*s,17.0*s,Color::from_rgb8(0x0f,0x17,0x2a)); self.round(ax+157.0*s,ay+265.0*s,338.0*s,208.0*s,Color::WHITE,12.0*s); self.round(ax+509.0*s,ay+265.0*s,176.0*s,208.0*s,Color::WHITE,12.0*s); self.round(ax+157.0*s,ay+490.0*s,528.0*s,170.0*s,Color::WHITE,12.0*s); if self.selected { let sx=ax+333.0*s; let sy=ay+134.0*s; self.stroke_round(sx,sy,174.0*s,117.0*s,p.accent,1.5,12.0*s); for (hx,hy) in [(sx-4.0,sy-4.0),(sx+170.0*s,sy-4.0),(sx-4.0,sy+113.0*s),(sx+170.0*s,sy+113.0*s)] { self.round(hx,hy,8.0,8.0,p.white,2.0); self.stroke_round(hx,hy,8.0,8.0,p.accent,1.0,2.0); } } }
    fn pixel_grid(&mut self,x:f64,y:f64,w:f64,h:f64){ let step=self.zoom; let c=Color::from_rgba8(255,255,255,24); let mut xx=x; while xx<x+w { self.line(xx,y,xx,y+h,c,0.5); xx+=step; } let mut yy=y; while yy<y+h { self.line(x,yy,x+w,yy,c,0.5); yy+=step; } }
    fn center_tool(&mut self,x:f64,ty:f64,kind:usize,shortcut:&str,active:bool,hover:bool,p:Palette){
        if active { self.round(x+4.0,ty+4.0,40.0,32.0,p.accent_soft,7.0); }
        else if hover { self.round(x+4.0,ty+4.0,40.0,32.0,p.surface,7.0); }
        let c=if active{p.accent}else if hover{p.text}else{p.muted};
        self.tool_icon(x+14.0,ty+10.0,kind,c); self.draw_text(shortcut,x+31.0,ty+20.0,7.0,p.faint);
    }
    fn layer_row(&mut self,x:f64,y:f64,s:&str,p:Palette,sel:bool,hover:bool){
        if sel { self.round(8.0,y-5.0,self.left_w()-16.0,26.0,p.accent_soft,6.0); }
        else if hover { self.round(8.0,y-5.0,self.left_w()-16.0,26.0,p.surface2,6.0); }
        self.draw_text(s,x+4.0,y+2.0,11.0,if sel||hover{p.text}else{p.muted});
    }
    fn section(&mut self,x:f64,y:f64,s:&str,p:Palette){ let width=self.right_w(); self.draw_text(s,x+16.0,y,9.0,p.faint); self.line(x+16.0,y+18.0,x+width-16.0,y+18.0,p.border,1.0); }
    fn prop(&mut self,x:f64,y:f64,name:&str,val:String,p:Palette){ let width=self.right_w(); self.draw_text(name,x+16.0,y+2.0,10.0,p.muted); self.round(x+98.0,y-6.0,width-114.0,28.0,p.field,6.0); self.draw_text(&val,x+108.0,y+2.0,10.0,p.text); }
    fn prop_pair(&mut self,x:f64,y:f64,n1:&str,v1:&str,n2:&str,v2:&str,p:Palette){ let width=self.right_w(); let half=(width-44.0)/2.0; self.draw_text(n1,x+16.0,y+2.0,9.5,p.faint); self.round(x+34.0,y-6.0,half-18.0,28.0,p.field,6.0); self.draw_text(v1,x+44.0,y+2.0,10.0,p.text); let sx=x+width/2.0; self.draw_text(n2,sx+2.0,y+2.0,9.5,p.faint); self.round(sx+20.0,y-6.0,half-18.0,28.0,p.field,6.0); self.draw_text(v2,sx+30.0,y+2.0,10.0,p.text); }

    fn command_palette(&mut self,p:Palette){ let w=self.config.width as f64; let h=self.config.height as f64; self.rect(0.0,0.0,w,h,Color::from_rgba8(0,0,0,145),0.0); let pw=480.0; let ph=350.0; let x=(w-pw)/2.0; let y=(h-ph)*0.22; self.round(x,y,pw,ph,p.surface2,10.0); self.stroke_round(x,y,pw,ph,p.border,1.0,10.0); self.round(x+12.0,y+12.0,pw-24.0,38.0,p.field,7.0); self.search_icon(x+26.0,y+24.0,p.muted); self.draw_text("Search every command and tool",x+46.0,y+23.0,11.0,p.muted); self.draw_text("CREATE",x+18.0,y+68.0,9.0,p.faint); for (i,(name,key)) in [("Frame / Section","F / S"),("Rectangle / Ellipse","R / O"),("Line / Pen","L / P"),("Polygon / Star / Triangle","Shape"),("Text / Image / Slice","T / I"),("Component / Instance","Cmd K"),("Prototype presentation","Play"),("Export selection","Shift E")].iter().enumerate(){ let ry=y+88.0+i as f64*31.0; if i==0 { self.round(x+10.0,ry-6.0,pw-20.0,28.0,p.accent_soft,6.0); } self.draw_text(name,x+22.0,ry+1.0,10.5,if i==0{p.text}else{p.muted}); self.draw_text(key,x+pw-76.0,ry+1.0,9.0,p.faint); } }

    fn search_icon(&mut self,x:f64,y:f64,c:Color){ let e=Ellipse::new((x,y),(9.0,9.0),0.0); self.scene.stroke(&vello::peniko::Stroke::new(1.4),&Affine::IDENTITY,c,None,&e); self.line(x+7.0,y+7.0,x+12.0,y+12.0,c,1.4); }
    fn align_icon(&mut self,x:f64,y:f64,kind:usize,c:Color){ if kind<3 { let anchor=x+[0.0,8.0,16.0][kind]; self.line(anchor,y,anchor,y+14.0,c,1.2); self.line(if kind==2{x+5.0}else{anchor},y+4.0,if kind==0{x+11.0}else{anchor},y+4.0,c,1.2); self.line(if kind==2{x+2.0}else{anchor},y+10.0,if kind==0{x+14.0}else{anchor},y+10.0,c,1.2); } else { let k=kind-3; let anchor=y+[0.0,7.0,14.0][k]; self.line(x,anchor,x+16.0,anchor,c,1.2); self.line(x+4.0,if k==2{y+4.0}else{anchor},x+4.0,if k==0{y+10.0}else{anchor},c,1.2); self.line(x+11.0,if k==2{y+2.0}else{anchor},x+11.0,if k==0{y+12.0}else{anchor},c,1.2); } }
    fn tool_icon(&mut self,x:f64,y:f64,kind:usize,c:Color){ match kind {
        0=>{ let mut p=BezPath::new(); p.move_to((x,y)); p.line_to((x+13.0,y+6.0)); p.line_to((x+7.0,y+8.0)); p.line_to((x+4.0,y+14.0)); p.close_path(); self.scene.stroke(&vello::peniko::Stroke::new(1.4),&Affine::IDENTITY,c,None,&p); }
        1=>{ self.line(x,y,x+5.0,y,c,1.3); self.line(x,y,x,y+5.0,c,1.3); self.line(x+11.0,y,x+16.0,y,c,1.3); self.line(x+16.0,y,x+16.0,y+5.0,c,1.3); self.line(x,y+11.0,x,y+16.0,c,1.3); self.line(x,y+16.0,x+5.0,y+16.0,c,1.3); self.line(x+11.0,y+16.0,x+16.0,y+16.0,c,1.3); self.line(x+16.0,y+11.0,x+16.0,y+16.0,c,1.3); }
        2=>{ self.stroke_round(x,y+2.0,16.0,13.0,c,1.3,2.0); self.line(x+4.0,y+2.0,x+4.0,y+15.0,c,1.0); }
        3=>self.stroke_round(x+1.0,y+1.0,14.0,14.0,c,1.3,2.0),
        4=>{ let e=Ellipse::new((x+8.0,y+8.0),(7.0,7.0),0.0); self.scene.stroke(&vello::peniko::Stroke::new(1.3),&Affine::IDENTITY,c,None,&e); }
        5=>{ let mut p=BezPath::new(); p.move_to((x+1.0,y+14.0)); p.curve_to((x+3.0,y+2.0),(x+10.0,y+2.0),(x+15.0,y+1.0)); self.scene.stroke(&vello::peniko::Stroke::new(1.4),&Affine::IDENTITY,c,None,&p); self.round(x+12.0,y-1.0,4.0,4.0,c,2.0); }
        6=>self.draw_text("T",x+3.0,y-2.0,16.0,c),
        7=>{ self.stroke_round(x,y+1.0,16.0,14.0,c,1.3,2.0); let mut p=BezPath::new(); p.move_to((x+2.0,y+13.0)); p.line_to((x+6.0,y+9.0)); p.line_to((x+9.0,y+12.0)); p.line_to((x+12.0,y+7.0)); p.line_to((x+15.0,y+11.0)); self.scene.stroke(&vello::peniko::Stroke::new(1.2),&Affine::IDENTITY,c,None,&p); }
        _=>{ let mut p=BezPath::new(); p.move_to((x+3.0,y+9.0)); p.line_to((x+3.0,y+3.0)); p.curve_to((x+3.0,y+1.0),(x+5.0,y+1.0),(x+5.0,y+3.0)); p.line_to((x+5.0,y+7.0)); p.line_to((x+7.0,y+1.0)); p.line_to((x+9.0,y+7.0)); p.line_to((x+11.0,y+2.0)); p.line_to((x+12.0,y+10.0)); p.curve_to((x+12.0,y+15.0),(x+4.0,y+16.0),(x+2.0,y+11.0)); self.scene.stroke(&vello::peniko::Stroke::new(1.3),&Affine::IDENTITY,c,None,&p); }
    }}

    fn neutral_art(&mut self,x:f64,y:f64,w:f64,h:f64,p:Palette){ let s=(w/820.0).min(h/650.0).min(1.0)*self.zoom; let aw=650.0*s; let ah=560.0*s; let ax=x+(w-aw)*0.46; let ay=y+(h-ah)*0.48; self.round(ax-5.0,ay+8.0,aw+10.0,ah+10.0,Color::from_rgba8(0,0,0,70),6.0); self.round(ax,ay,aw,ah,Color::WHITE,2.0); self.round(ax+32.0*s,ay+32.0*s,586.0*s,44.0*s,Color::from_rgb8(0xf4,0xf4,0xf5),6.0*s); self.round(ax+48.0*s,ay+46.0*s,72.0*s,12.0*s,Color::from_rgb8(0xd4,0xd4,0xd8),3.0*s); self.round(ax+32.0*s,ay+100.0*s,586.0*s,252.0*s,Color::from_rgb8(0xfa,0xfa,0xfa),12.0*s); self.round(ax+68.0*s,ay+138.0*s,224.0*s,18.0*s,Color::from_rgb8(0xd4,0xd4,0xd8),4.0*s); self.round(ax+68.0*s,ay+172.0*s,302.0*s,10.0*s,Color::from_rgb8(0xe4,0xe4,0xe7),3.0*s); self.round(ax+68.0*s,ay+190.0*s,254.0*s,10.0*s,Color::from_rgb8(0xe4,0xe4,0xe7),3.0*s); self.round(ax+68.0*s,ay+230.0*s,112.0*s,34.0*s,Color::from_rgb8(0x18,0x18,0x1b),7.0*s); self.round(ax+399.0*s,ay+132.0*s,174.0*s,172.0*s,Color::from_rgb8(0xf0,0xf0,0xf2),10.0*s); for i in 0..3 { self.round(ax+(32.0+i as f64*203.0)*s,ay+376.0*s,180.0*s,140.0*s,Color::from_rgb8(0xfa,0xfa,0xfa),10.0*s); } if self.selected { let sx=ax+28.0*s; let sy=ay+96.0*s; self.stroke_round(sx,sy,594.0*s,260.0*s,p.accent,1.5,14.0*s); for (hx,hy) in [(sx-4.0,sy-4.0),(sx+590.0*s,sy-4.0),(sx-4.0,sy+256.0*s),(sx+590.0*s,sy+256.0*s)] { self.round(hx,hy,8.0,8.0,p.white,2.0); self.stroke_round(hx,hy,8.0,8.0,p.accent,1.0,2.0); } } }
    fn rect(&mut self,x:f64,y:f64,w:f64,h:f64,c:Color,_r:f64){ self.scene.fill(vello::peniko::Fill::NonZero,&Affine::IDENTITY, c, None, &Rect::new(x,y,x+w,y+h)); }
    fn round(&mut self,x:f64,y:f64,w:f64,h:f64,c:Color,r:f64){ let rr=RoundedRect::new(x,y,x+w,y+h,r); self.scene.fill(vello::peniko::Fill::NonZero,&Affine::IDENTITY,c,None,&rr); }
    fn stroke_round(&mut self,x:f64,y:f64,w:f64,h:f64,c:Color,width:f64,r:f64){ let rr=RoundedRect::new(x,y,x+w,y+h,r); self.scene.stroke(&vello::peniko::Stroke::new(width),&Affine::IDENTITY,c,None,&rr); }
    fn line(&mut self,x1:f64,y1:f64,x2:f64,y2:f64,c:Color,width:f64){ let mut path=BezPath::new(); path.move_to(Point::new(x1,y1)); path.line_to(Point::new(x2,y2)); self.scene.stroke(&vello::peniko::Stroke::new(width),&Affine::IDENTITY,c,None,&path); }
    fn draw_text(&mut self,text:&str,x:f64,y:f64,size:f64,c:Color){ let spans=[Span::new(text,size).color(c).font(self.font)]; let style=TextBlockStyle{max_width:1000.0,line_height:1.2,align:Align::Left,wrap:x_native::TextWrap::NoWrap}; let _=encode_rich_text(&mut self.scene,&self.fonts,&spans,self.font,Affine::translate((x,y)),&style); }
}
