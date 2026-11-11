-- name: 2048
-- kind: game
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: Slide the tiles and join equal ones.
-- about.de: Schiebe die Kacheln und verbinde gleiche.
-- license: MIT

-- The board is a list of sixteen numbers, row by row, 0 for a free cell. The game is saved
-- after every move, so switching off loses nothing.

local board, score, best, reached, over, back, fresh, just_reached

local function de() return lang() == "de" end

local function labels()
  local undo = ""
  if back then undo = de() and "Zurück" or "Undo" end
  soft({de() and "Neu" or "New", undo})
end

-- A 2 (nine times in ten) or a 4 on a free cell; it is outlined until the next move.
local function add()
  local free = {}
  for i = 1, 16 do
    if board[i] == 0 then free[#free + 1] = i end
  end
  if #free == 0 then return end
  local i = free[math.random(#free)]
  board[i] = math.random(10) <= 9 and 2 or 4
  fresh = i
end

-- A new game; the best score stays.
local function start()
  board = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0}
  score = 0
  reached = false
  over = false
  back = nil
  fresh = nil
  add()
  add()
end

-- The cells of line `k` (0 to 3) in the order the tiles slide towards: the first is where
-- they go.
local function cells(dir, k)
  if dir == "left" then return {k * 4 + 1, k * 4 + 2, k * 4 + 3, k * 4 + 4}
  elseif dir == "right" then return {k * 4 + 4, k * 4 + 3, k * 4 + 2, k * 4 + 1}
  elseif dir == "up" then return {k + 1, k + 5, k + 9, k + 13}
  else return {k + 13, k + 9, k + 5, k + 1} end
end

-- The numbers of a line slid to its start, equal neighbours joined once: the new line and the
-- points the joins give. {2, 2, 2, 2} gives {4, 4, 0, 0} and 8.
function slide(line)
  local tiles = {}
  for _, v in ipairs(line) do
    if v ~= 0 then tiles[#tiles + 1] = v end
  end
  local out, gained, i = {}, 0, 1
  while i <= #tiles do
    if i < #tiles and tiles[i] == tiles[i + 1] then
      out[#out + 1] = tiles[i] * 2
      gained = gained + tiles[i] * 2
      i = i + 2
    else
      out[#out + 1] = tiles[i]
      i = i + 1
    end
  end
  while #out < #line do out[#out + 1] = 0 end
  return out, gained
end

-- The board after a move in `dir`, the points, and whether anything moved. Global, like
-- `slide`, so that the tests can call them.
function moved(from, dir)
  local next, gained, changed = {}, 0, false
  for k = 0, 3 do
    local at = cells(dir, k)
    local line = {}
    for j, i in ipairs(at) do line[j] = from[i] end
    local out, points = slide(line)
    for j = 1, 4 do
      next[at[j]] = out[j]
      if out[j] ~= from[at[j]] then changed = true end
    end
    gained = gained + points
  end
  return next, gained, changed
end

local function moves_left()
  for _, dir in ipairs({"left", "right", "up", "down"}) do
    local _, _, changed = moved(board, dir)
    if changed then return true end
  end
  return false
end

local function keep()
  save({board = board, score = score, best = best, reached = reached})
end

local function turn(dir)
  if over then return end
  local next, gained, changed = moved(board, dir)
  -- A move that changes nothing is ignored, and adds no tile.
  if not changed then return end
  back = {board = board, score = score}
  board = next
  score = score + gained
  if score > best then best = score end
  add()
  if not reached then
    for i = 1, 16 do
      if board[i] == 2048 then
        reached = true
        just_reached = true
      end
    end
  end
  over = not moves_left()
  keep()
  labels()
end

function init()
  local saved = load()
  best = 0
  if type(saved) == "table" and type(saved.board) == "table" and #saved.board == 16 then
    board = saved.board
    score = saved.score or 0
    best = saved.best or 0
    reached = saved.reached == true
    back = nil
    fresh = nil
    over = not moves_left()
  else
    start()
  end
  labels()
end

local directions = {left = "left", right = "right", up = "up", down = "down", ["4"] = "left", ["6"] = "right", ["8"] = "up", ["2"] = "down"}

function key(k)
  just_reached = false
  if directions[k] then
    turn(directions[k])
  elseif k == "soft1" then
    -- Asks first while a game is going on.
    local running = score > 0 and not over
    if not running or confirm(de() and "Neues Spiel?" or "New game?") then
      start()
      keep()
      labels()
    end
  elseif k == "soft2" and back then
    -- One move back, once.
    board = back.board
    score = back.score
    back = nil
    fresh = nil
    over = false
    keep()
    labels()
  end
end

function leave() keep() end

-- The tile palette by the tile's power of two.
local function colour(v)
  local n = 0
  while v > 1 do
    v = v // 2
    n = n + 1
  end
  return "tile" .. ((n - 1) % 14 + 1)
end

local function side_panel()
  local s = side()
  local room = s.w - 8
  local x, y = s.x + 4, s.y + 4
  local small = text_small_height() + 4
  local points = de() and "Punkte" or "Score"
  local best_name = de() and "Bestwert" or "Best"
  if s.h >= 2 * (text_height() + small) + 8 then
    -- Beside the board: each number under its name.
    text_small(x, y, points, "muted")
    y = y + small
    text(x, y, score, "text")
    y = y + text_height() + 6
    text_small(x, y, best_name, "muted")
    y = y + small
    text(x, y, best, "text")
    y = y + text_height() + 10
  else
    -- Below it: one line.
    local line = points .. " " .. score .. "   " .. best_name .. " " .. best
    if text_small_width(line) <= room then text_small(x, y, line, "text") end
    y = y + small + 2
  end
  local status = ""
  if over then
    status = de() and "Keine Züge mehr" or "No moves left"
  elseif just_reached then
    status = de() and "2048 erreicht!" or "2048 reached!"
  end
  if status ~= "" and text_small_width(status) <= room and y + small <= s.y + s.h then
    text_small(x, y, status, "amber")
  end
end

function draw()
  clear("background")
  local g = grid(4, 4)
  local gap = g.size // 20 + 2
  round(g.x, g.y, g.w, g.h, 8, "surface")
  for i = 1, 16 do
    local c = g:cell((i - 1) % 4 + 1, (i - 1) // 4 + 1)
    local x, y = c.x + gap, c.y + gap
    local w, h = c.w - 2 * gap, c.h - 2 * gap
    local v = board[i]
    if v == 0 then
      round(x, y, w, h, 6, "hairline")
    else
      round(x, y, w, h, 6, colour(v))
      if i == fresh then frame(x, y, w, h, 6, "text") end
      local s = tostring(v)
      -- Four digits and more in the small font.
      if #s >= 4 then
        text_small(x + (w - text_small_width(s)) // 2, y + (h - text_small_height()) // 2, s, "text")
      else
        text(x + (w - text_width(s)) // 2, y + (h - text_height()) // 2, s, "text")
      end
    end
  end
  side_panel()
end
