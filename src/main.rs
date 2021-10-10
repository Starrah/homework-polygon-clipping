#[macro_use]
extern crate glium;

use glium::{Display, Frame, glutin, Program, Surface};
use glium::glutin::dpi::PhysicalPosition;

#[derive(Copy, Clone)]
struct Point {
    position: [f32; 2],
}
implement_vertex!(Point, position);
impl Point {
    fn new(position: [f32; 2]) -> Point {
        Point { position: position }
    }
}

type Polygen = Vec<Vec<Point>>;
type Path = Vec<Point>;
type Line = [Point; 2];

const VERTEX_SHADER_SRC: &str = r#"
        #version 140

        uniform vec2 window_size;

        in vec2 position;

        void main() {
            gl_Position = vec4(position / window_size * vec2(2, -2) - vec2(1, -1), 0.0, 1.0);
        }
    "#;

const FRAGMENT_SHADER_SRC: &str = r#"
        #version 140

        uniform vec4 color2;

        out vec4 color;

        void main() {
            color = color2;
        }
    "#;

const MAIN_STATUS_TEXT: &str = "请输入主多边形。右键闭合，回车完成，backspace清除";
const CLIPPER_STATUS_TEXT: &str = "请输入裁剪多边形。右键闭合，回车完成，backspace清除";
const RESULT_STATUS_TEXT: &str = "结果展示。回车开始下一轮输入。红-裁剪结果，绿-主多边形，蓝：裁剪多边形";

fn main() {
    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new();
    let cb = glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    let program = glium::Program::from_source(&display, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None).unwrap();

    let mut mouse_position = PhysicalPosition { x: 0.0, y: 0.0 };

    // 多边形的表达：点的列表的列表
    let mut main_polygon = vec!(Vec::new());
    let mut clipper = vec!(Vec::new());

    #[derive(Copy, Clone)]
    enum Status { Main, Clipper, Result }
    let mut status = Status::Main;

    event_loop.run(move |event, _, control_flow| {
        let get_status_str = |status: Status| -> &str {
            match status {
                Status::Main => MAIN_STATUS_TEXT,
                Status::Clipper => CLIPPER_STATUS_TEXT,
                Status::Result => RESULT_STATUS_TEXT,
            }
        };

        let set_status_str = |status: Status| {
            display.gl_window().window().set_title(get_status_str(status));
        };

        let _check_last_edge_valid = |polygen: &mut Polygen, closed: bool| -> bool {
            let last_path = polygen.last_mut().unwrap();
            let last_path_len = last_path.len();
            if last_path_len < 2 { return true; }
            let last_edge = [last_path[last_path_len - 2], last_path[last_path_len - 1]];
            if polygen.len() > 1 {
                for (i, path) in polygen.iter().enumerate() {
                    if i >= polygen.len() - 1 { continue; }
                    for i in 0..path.len() - 1 {
                        let edge = [path[i], path[i + 1]];
                        let inter = intersection(&edge, &last_edge);
                        if let Some(_) = inter { return false; }
                    }
                }
            }
            let path = polygen.last_mut().unwrap();
            let start_idx = if closed { 1 } else { 0 };
            let end_idx = path.len() as isize - 3;
            if end_idx >= start_idx {
                for i in start_idx..end_idx {
                    let i = i as usize;
                    let edge = [path[i], path[i + 1]];
                    let inter = intersection(&edge, &last_edge);
                    if let Some(_) = inter { return false; }
                }
            }
            return true;
        };

        let check_last_edge_valid = |polygen: &mut Polygen, closed: bool| -> bool {
            let r = _check_last_edge_valid(polygen, closed);
            if !r { polygen.last_mut().unwrap().pop(); }
            r
        };

        let is_path_anti_clockwise = |path: &Path| -> bool {
            let mut result = 0.0;
            for i in 0..path.len() - 1 {
                result += (path[i + 1].position[0] - path[i].position[0]) * (path[i + 1].position[1] + path[i].position[1]);
            }
            return result > 0.0;
        };

        let add_point = |polygen: &mut Polygen, point: Point| -> Option<&str> {
            polygen.last_mut().unwrap().push(point);
            let check_result = check_last_edge_valid(polygen, false);
            return if check_result { None } else { Some("输入的点所构成的边将与已有边相交！请重新点击！") }; // 没有错误
        };

        let close_path = |polygen: &mut Polygen| -> Option<&str> {
            let current_path = polygen.last_mut().unwrap();
            if current_path.len() > 2 {
                current_path.push(current_path[0]);
                let check_result = check_last_edge_valid(polygen, true);
                return if check_result {
                    polygen.push(Vec::new());
                    None
                } else { Some("操作无效，此时闭合产生的边将与已有边相交！") };
            } else {
                current_path.clear();
                return Some("已点选的点数小于3，无法构成回路！请重新输入！"); // 有错误
            }
        };

        let finish_polygen = |polygen: &mut Polygen| -> Option<&str> {
            let mut r = close_path(polygen);
            if polygen.last_mut().unwrap().len() == 0 { r = None; }
            if let None = r { polygen.pop(); } // 弹出位于尾部的空path
            r
        };

        let repaint = |main_polygen, clipper, status| {
            let window_size = display.gl_window().window().inner_size();
            let mut frame = display.draw();
            frame.clear_color(0.0, 0.0, 0.0, 0.0);

            let mut paint_polygen = |polygen: &Polygen, color: [f32; 4]| {
                let uniform = uniform! {
                    window_size: [window_size.width as f32, window_size.height as f32],
                    color2: color,
                };
                for path in polygen {
                    paint_path(&display, &program, &uniform, &mut frame, &path);
                }
            };

            match status {
                Status::Main => {
                    paint_polygen(main_polygen, [0.0, 1.0, 0.0, 1.0]);
                }
                Status::Clipper => {
                    paint_polygen(main_polygen, [0.0, 1.0, 0.0, 1.0]);
                    paint_polygen(clipper, [0.0, 1.0, 1.0, 1.0]);
                }
                Status::Result => {
                    let clip_result = clipping(main_polygen, clipper);
                    paint_polygen(&clip_result.main, [0.0, 1.0, 0.0, 1.0]);
                    paint_polygen(&clip_result.clipper, [0.0, 1.0, 1.0, 1.0]);
                    paint_polygen(&clip_result.result, [1.0, 0.0, 0.0, 1.0]);
                }
            }

            frame.finish().unwrap();
        };

        let next_frame_time = std::time::Instant::now() +
            std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                }
                glutin::event::WindowEvent::MouseInput { state, button, .. } => {
                    let point = Point::new([mouse_position.x as f32, mouse_position.y as f32]);
                    let polygon = match status {
                        Status::Main => &mut main_polygon,
                        Status::Clipper => &mut clipper,
                        _ => return,
                    };
                    if state == glium::glutin::event::ElementState::Released {
                        let mut r;
                        const ANTI_CLOCKWISE_STR: &str = "闭合成功！(您刚画的回路是逆时针的) ";
                        const CLOCKWISE_STR: &str = "闭合成功！(您刚画的回路是顺时针的) ";
                        let mut empty_str = String::from("");
                        if button == glium::glutin::event::MouseButton::Left {
                            r = add_point(polygon, point);
                        } else if button == glium::glutin::event::MouseButton::Right {
                            r = close_path(polygon);
                            if let None = r {
                                let anti_clockwise = is_path_anti_clockwise(&polygon[polygon.len() - 2]);
                                let s = if anti_clockwise {ANTI_CLOCKWISE_STR} else {CLOCKWISE_STR};
                                empty_str += s;
                                empty_str += get_status_str(status);
                                r = Some(&empty_str);
                            }
                        } else { return; }
                        if let Some(err_str) = r { display.gl_window().window().set_title(err_str); } else { set_status_str(status); }
                        repaint(&main_polygon, &clipper, status);
                    };
                }
                glutin::event::WindowEvent::CursorMoved { position, .. } => {
                    mouse_position = position;
                    return;
                }
                _ => return,
            },
            glutin::event::Event::DeviceEvent { event, .. } => match event {
                glutin::event::DeviceEvent::Key(key) => {
                    if key.state == glium::glutin::event::ElementState::Released {
                        if let Some(key_code) = key.virtual_keycode {
                            match key_code {
                                glium::glutin::event::VirtualKeyCode::Return => {
                                    let r;
                                    status = match status {
                                        Status::Main => {
                                            r = finish_polygen(&mut main_polygon);
                                            if let None = r { Status::Clipper } else { Status::Main }
                                        }
                                        Status::Clipper => {
                                            r = finish_polygen(&mut clipper);
                                            if let None = r { Status::Result } else { Status::Clipper }
                                        }
                                        Status::Result => {
                                            r = None;
                                            main_polygon = vec!(Vec::new());
                                            clipper = vec!(Vec::new());
                                            Status::Main
                                        }
                                    };
                                    if let Some(err_str) = r { display.gl_window().window().set_title(err_str); } else { set_status_str(status); }
                                }
                                glium::glutin::event::VirtualKeyCode::Back => {
                                    status = match status {
                                        Status::Main => {
                                            main_polygon = vec!(Vec::new());
                                            display.gl_window().window().set_title("主多边形已清空！");
                                            Status::Main
                                        }
                                        Status::Clipper => {
                                            clipper = vec!(Vec::new());
                                            display.gl_window().window().set_title("裁剪多边形已清空！");
                                            Status::Clipper
                                        }
                                        Status::Result => {
                                            main_polygon = vec!(Vec::new());
                                            clipper = vec!(Vec::new());
                                            set_status_str(Status::Main);
                                            Status::Main
                                        }
                                    }
                                }
                                _ => return,
                            }
                            repaint(&main_polygon, &clipper, status);
                        }
                    }
                }
                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::Init => {
                    repaint(&main_polygon, &clipper, status);
                }
                _ => return,
            },
            _ => return,
        }
    });
}

fn paint_path(display: &Display, program: &Program, uniform: &impl glium::uniforms::Uniforms, frame: &mut Frame, path: &Path) {
    let vertex_buffer = glium::VertexBuffer::new(display, &path).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::LineStrip);
    frame.draw(&vertex_buffer, &indices, &program, uniform,
               &glium::DrawParameters {
                   line_width: Some(5.0),
                   ..Default::default()
               }).unwrap();
}


struct ClipResult {
    main: Polygen,
    clipper: Polygen,
    result: Polygen,
}

fn intersection(l1: &Line, l2: &Line) -> Option<(Point, f32, f32, PointType)> {
    let a = l1[0].position;
    let b = l1[1].position;
    let c = l2[0].position;
    let d = l2[1].position;
    let dir1 = [b[0] - a[0], b[1] - a[1]];
    let dir2 = [d[0] - c[0], d[1] - c[1]];
    #[allow(non_snake_case)] let D = a[0] * dir2[1] - b[0] * dir2[1] - c[0] * dir1[1] + d[0] * dir1[1];
    if D == 0.0 { return None; } // 两直线平行则没有交点；重合也视为没有交点
    let s = (a[0] * dir2[1] + c[0] * (a[1] - d[1]) + d[0] * (c[1] - a[1])) / D;
    let t = -(a[0] * (c[1] - b[1]) + b[0] * (a[1] - c[1]) + c[0] * dir1[1]) / D;
    if s >= 0.0 && s <= 1.0 && t >= 0.0 && t <= 1.0 {
        // 根据叉乘推导出的
        let m = dir2[0] * dir1[1] - dir1[0] * dir2[1];
        let point_type = if m > 0.0 { PointType::Out } else { PointType::In };
        Some((Point::new([a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1])]), s, t, point_type))
    } else { None }
}

#[derive(Clone)]
enum PointType { OriginMain, OriginClipper, In, Out }

#[derive(Clone)]
struct PointTableItem {
    point: Point,
    point_type: PointType,
    param1: f32,
    param2: f32,
    next1: usize,
    next2: usize,
    is_result: bool,
}

fn clipping(polygen: &Polygen, clipper: &Polygen) -> ClipResult {
    // 构建初始顶点表
    let mut table = Vec::new();
    let generate_point_table = |table: &mut Vec<PointTableItem>, polygen: &Polygen, point_type: PointType| {
        for path in polygen {
            let head_index = table.len();
            for (i, point) in path[..path.len() - 1].iter().enumerate() {
                let next_index = if i < path.len() - 2 { table.len() + 1 } else { head_index };
                table.push(PointTableItem {
                    point: point.clone(),
                    point_type: point_type.clone(),
                    param1: 1.0,
                    param2: 1.0,
                    next1: if let PointType::OriginMain = point_type { next_index } else { 0 },
                    next2: if let PointType::OriginClipper = point_type { next_index } else { 0 },
                    is_result: false,
                });
            }
        }
    };
    generate_point_table(&mut table, polygen, PointType::OriginMain);
    let main_end = table.len();
    generate_point_table(&mut table, clipper, PointType::OriginClipper);
    let clipper_end = table.len();
    let table_origin = table.clone();

    // 寻找交点，插入顶点表
    for i1 in 0..main_end {
        for i2 in main_end..clipper_end {
            let item1 = &table_origin[i1];
            let item2 = &table_origin[i2];
            let l1 = [item1.point, table_origin[item1.next1].point];
            let l2 = [item2.point, table_origin[item2.next2].point];
            let inter = intersection(&l1, &l2);
            if let Some((pt, s, t, point_type)) = inter {
                let mut new_item = PointTableItem { point: pt.clone(), point_type: point_type.clone(), param1: s, param2: t, next1: 0, next2: 0, is_result: false };
                // 寻找插入位置
                let mut cur = i1;
                while table[table[cur].next1].param1 < s {
                    cur = table[cur].next1;
                }
                new_item.next1 = table[cur].next1;
                table[cur].next1 = table.len();

                let mut cur = i2;
                while table[table[cur].next2].param2 < t {
                    cur = table[cur].next2;
                }
                new_item.next2 = table[cur].next2;
                table[cur].next2 = table.len();

                table.push(new_item);
            }
        }
    }

    // 运行算法
    let mut result = Vec::new();
    loop {
        let mut start = None;
        // 寻找没到达过的交点
        for (i, item) in table.iter().enumerate() {
            match item.point_type {
                PointType::OriginMain => (),
                PointType::OriginClipper => (),
                _ => {
                    if !item.is_result {
                        start = Some(i);
                        break;
                    }
                }
            }
        }
        if let None = start { break; } // 找不到未处理的交点，算法完成
        let start = start.unwrap();

        let mut res = Vec::new();
        let mut cur = start;
        loop {
            let item = &mut table[cur];
            item.is_result = true;
            res.push(item.point);
            cur = match item.point_type {
                PointType::OriginMain => item.next1,
                PointType::OriginClipper => item.next2,
                PointType::In => item.next1,
                PointType::Out => item.next2,
            };
            if cur == start {
                res.push(table[start].point);
                break;
            }
        }
        result.push(res);
    };

    // 造访所有点，计算新的polygen和clipper
    // 沿着主多边形顶点表，在每个环上走一次
    let mut polygen = Vec::new();
    loop {
        let mut start = None;
        // 寻找没到达过的顶点
        for (i, item) in table[0..main_end].iter().enumerate() {
            match item.point_type {
                PointType::OriginMain => {
                    if !item.is_result {
                        start = Some(i);
                        break;
                    }
                }
                _ => ()
            }
        }
        if let None = start { break; } // 找不到未到达的顶点，搜索完成
        let start = start.unwrap();

        let mut res = Vec::new();
        let mut cur = start;
        loop {
            let item = &mut table[cur];
            let is_edge = match item.point_type {
                PointType::OriginMain => !item.is_result,
                PointType::OriginClipper => {
                    debug_assert!(false);
                    false
                }
                PointType::In => false,
                PointType::Out => true,
            };
            item.is_result = true;
            let next_ptr = item.next1;

            if is_edge {
                if res.len() == 0 { res.push(item.point) }
                res.push(table[next_ptr].point)
            } else {
                if res.len() > 0 {
                    polygen.push(res);
                    res = Vec::new();
                }
            }
            cur = next_ptr;
            if cur == start {
                if res.len() > 0 {
                    polygen.push(res);
                }
                break;
            }
        };
    }

    let mut clipper = Vec::new();
    loop {
        let mut start = None;
        // 寻找没到达过的顶点
        for (i, item) in table[main_end..clipper_end].iter().enumerate() {
            match item.point_type {
                PointType::OriginClipper => {
                    if !item.is_result {
                        start = Some(i);
                        break;
                    }
                }
                _ => ()
            }
        }
        if let None = start { break; } // 找不到未到达的顶点，搜索完成
        let start = start.unwrap() + main_end;

        let mut res = Vec::new();
        let mut cur = start;

        loop {
            let item = &mut table[cur];
            let is_edge = match item.point_type {
                PointType::OriginMain => {
                    debug_assert!(false);
                    false
                }
                PointType::OriginClipper => !item.is_result,
                PointType::In => true,
                PointType::Out => false,
            };
            item.is_result = true;
            let next_ptr = item.next2;

            if is_edge {
                if res.len() == 0 { res.push(item.point) }
                res.push(table[next_ptr].point)
            } else {
                if res.len() > 0 {
                    clipper.push(res);
                    res = Vec::new();
                }
            }
            cur = next_ptr;
            if cur == start {
                if res.len() > 0 {
                    clipper.push(res);
                }
                break;
            }
        };
    }

    ClipResult { result, main: polygen, clipper }
}