let mut pos_x = 0;
let mut pos_y = 0;

if Window::get_key(&surface.window, Key::A) == Action::Press
    || Window::get_key(&surface.window, Key::A) == Action::Repeat
{
    pos_x -= 1;
}
if Window::get_key(&surface.window, Key::D) == Action::Press
    || Window::get_key(&surface.window, Key::D) == Action::Repeat
{
    pos_x += 1;
}
if Window::get_key(&surface.window, Key::W) == Action::Press
    || Window::get_key(&surface.window, Key::W) == Action::Repeat
{
    pos_y -= 1;
}
if Window::get_key(&surface.window, Key::S) == Action::Press
    || Window::get_key(&surface.window, Key::S) == Action::Repeat
{
    pos_y += 1
}