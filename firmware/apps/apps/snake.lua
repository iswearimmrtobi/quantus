-- name: Snake
-- kind: game
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: Steer the snake to the food; it grows, and must not hit the wall or itself.
-- about.de: Lenke die Schlange zum Futter; sie wächst und darf weder Wand noch sich selbst treffen.
-- license: MIT

-- The snake is a list of cell indices, row by row, its head first. It steps on the timer,
-- every 150 ms at first and faster as it grows, down to the timer's limit of 100 ms. A step
-- changes only the head, the tail and the food, so only a few bands of the screen are sent.

local COLS, ROWS = 16, 12
local START_LENGTH = 3

local DIRS = {up = {0, -1}, down = {0, 1}, left = {-1, 0}, right = {1, 0}}
local OPPOSITE = {up = "down", down = "up", left = "right", right = "left"}
local KEYS = {up = "up", down = "down", left = "left", right = "right", ["8"] = "up", ["2"] = "down", ["4"] = "left", ["6"] = "right"}

local s, queue, over, paused, started, best, interval

local function de() return lang() == "de" end

local function labels()
  local pause = ""
  if started and not over then
    if paused then pause = de() and "Weiter" or "Resume" else pause = "Pause" end
  end
  soft({de() and "Neu" or "New", pause})
end

-- Adds a turn to `q`, the turns to make at the next steps, unless it repeats or reverses the
-- last one (or the heading, when there is none) or two are waiting already.
function steer(q, dir, heading)
  local last = q[#q] or heading
  if #q >= 2 or dir == last or dir == OPPOSITE[last] then return end
  q[#q + 1] = dir
end

-- One step of snake `g` (`cols`, `rows`, `body`, `dir`, `food`): "crash" when it hits the
-- wall or itself, and then it stays as it was; "ate" when the head reached the food and the
-- snake grew; else "moved".
function advance(g)
  local head = g.body[1]
  local col, row = (head - 1) % g.cols, (head - 1) // g.cols
  local d = DIRS[g.dir]
  local nc, nr = col + d[1], row + d[2]
  if nc < 0 or nc >= g.cols or nr < 0 or nr >= g.rows then return "crash" end
  local to = nr * g.cols + nc + 1
  local eats = to == g.food
  -- The tail moves on unless the snake grows, so the head may take its cell.
  local last = eats and #g.body or #g.body - 1
  for k = 1, last do
    if g.body[k] == to then return "crash" end
  end
  table.insert(g.body, 1, to)
  if eats then return "ate" end
  table.remove(g.body)
  return "moved"
end

-- A free cell for the food, or nil when the snake fills the board.
function food_for(g)
  local taken = {}
  for _, i in ipairs(g.body) do taken[i] = true end
  local free = {}
  for i = 1, g.cols * g.rows do
    if not taken[i] then free[#free + 1] = i end
  end
  if #free == 0 then return nil end
  return free[math.random(#free)]
end

local function score() return #s.body - START_LENGTH end

local function speed() return math.max(100, 150 - 2 * score()) end

local function keep() save({best = best}) end

-- A new snake in the middle, heading right, waiting for the first arrow.
local function start()
  local row = ROWS // 2
  local body = {}
  for k = 0, START_LENGTH - 1 do body[#body + 1] = row * COLS + COLS // 2 - k end
  s = {cols = COLS, rows = ROWS, body = body, dir = "right"}
  s.food = food_for(s)
  queue = {}
  over, paused, started = nil, false, false
  interval = speed()
  timer(0)
  labels()
end

function init()
  local saved = load()
  best = type(saved) == "table" and math.type(saved.best) == "integer" and saved.best or 0
  start()
end

local function finish(result)
  over = result
  timer(0)
  if score() > best then
    best = score()
    keep()
  end
  labels()
end

function tick()
  if not started or paused or over then return end
  if #queue > 0 then s.dir = table.remove(queue, 1) end
  local result = advance(s)
  if result == "crash" then
    finish("crash")
  elseif result == "ate" then
    s.food = food_for(s)
    if not s.food then
      finish("won")
      return
    end
    if speed() ~= interval then
      interval = speed()
      timer(interval)
    end
  end
end

function key(k)
  if k == "soft1" then
    start()
  elseif k == "soft2" then
    if started and not over then
      paused = not paused
      timer(paused and 0 or interval)
      labels()
    end
  elseif over then
    -- Any other key after the end sets up the next game.
    start()
  elseif KEYS[k] then
    if paused then
      paused = false
      timer(interval)
    end
    steer(queue, KEYS[k], s.dir)
    if not started then
      started = true
      timer(interval)
      labels()
    end
  end
end

function leave() keep() end

local function side_panel()
  local sd = side()
  local room = sd.w - 8
  local x, y = sd.x + 4, sd.y + 4
  local small = text_small_height() + 4
  local points = de() and "Punkte" or "Score"
  local best_name = de() and "Bestwert" or "Best"
  local status, colour = "", "muted"
  if over == "won" then
    status, colour = de() and "Geschafft!" or "Board full!", "amber"
  elseif over then
    status, colour = de() and "Vorbei" or "Game over", "error"
  elseif paused then
    status = "Pause"
  elseif not started then
    status = de() and "Pfeil: Start" or "Arrow: start"
  end
  if sd.h >= 2 * (text_height() + small) + small + 8 then
    text_small(x, y, points, "muted")
    y = y + small
    text(x, y, score(), "text")
    y = y + text_height() + 6
    text_small(x, y, best_name, "muted")
    y = y + small
    text(x, y, best, "text")
    y = y + text_height() + 10
  else
    local line = points .. " " .. score() .. "   " .. best_name .. " " .. best
    if status ~= "" then line = line .. "   " .. status end
    if text_small_width(line) <= room and y + small <= sd.y + sd.h then
      text_small(x, y, line, over and colour or "text")
    end
    return
  end
  if status ~= "" and text_small_width(status) <= room and y + small <= sd.y + sd.h then
    text_small(x, y, status, colour)
  end
end

function draw()
  clear("background")
  local g = grid(COLS, ROWS)
  round(g.x, g.y, g.w, g.h, 6, "surface")
  if s.food then
    local c = g:cell((s.food - 1) % COLS + 1, (s.food - 1) // COLS + 1)
    circle(c.x + 2, c.y + 2, c.w - 4, "amber")
  end
  for k = #s.body, 1, -1 do
    local i = s.body[k]
    local c = g:cell((i - 1) % COLS + 1, (i - 1) // COLS + 1)
    local colour = "cyan"
    if k == 1 then colour = over == "crash" and "error" or "result" end
    round(c.x + 1, c.y + 1, c.w - 2, c.h - 2, 3, colour)
  end
  side_panel()
end
