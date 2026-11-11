-- name: Minesweeper
-- kind: game
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: Open every cell without a mine; the numbers count the mines around.
-- about.de: Decke alle Felder ohne Mine auf; die Zahlen zählen die Minen ringsum.
-- license: MIT

-- The board is three lists, row by row: `mines` (1 for a mine), `counts` (the mines around
-- each cell) and `state` (0 hidden, 1 open, 2 flagged). The mines are laid on the first
-- reveal, away from that cell and its neighbours, so the first reveal never loses.

local LEVELS = {{9, 9, 10}, {16, 12, 30}, {20, 12, 45}}

local level, cols, rows, n, mines, counts, state, placed, over, boom, flagging, best
local base, mark, last, elapsed

local function de() return lang() == "de" end

local function level_name(k)
  if de() then return ({"leicht", "mittel", "schwer"})[k] end
  return ({"easy", "medium", "hard"})[k]
end

local function labels()
  local mode
  if flagging then mode = de() and "Flagge" or "Flag" else mode = de() and "Öffnen" or "Open" end
  soft({de() and "Neu" or "New", level_name(level), mode})
end

-- The cells around cell `i` of a board `c` × `r`.
function neighbours(i, c, r)
  local list = {}
  local col, row = (i - 1) % c, (i - 1) // c
  for dr = -1, 1 do
    for dc = -1, 1 do
      local x, y = col + dc, row + dr
      if (dr ~= 0 or dc ~= 0) and x >= 0 and x < c and y >= 0 and y < r then
        list[#list + 1] = y * c + x + 1
      end
    end
  end
  return list
end

-- `count` mines at random on a board `c` × `r`, none on `safe` or around it (none on `safe`
-- alone when the board has no room for that): a list of 0 and 1.
function place(c, r, count, safe)
  local keep_clear = {[safe] = true}
  local around = neighbours(safe, c, r)
  if c * r - 1 - #around >= count then
    for _, j in ipairs(around) do keep_clear[j] = true end
  end
  local free = {}
  for i = 1, c * r do
    if not keep_clear[i] then free[#free + 1] = i end
  end
  local laid = {}
  for i = 1, c * r do laid[i] = 0 end
  for k = 1, math.min(count, #free) do
    local j = math.random(k, #free)
    free[k], free[j] = free[j], free[k]
    laid[free[k]] = 1
  end
  return laid
end

-- The mines around each cell.
function count(m, c, r)
  local out = {}
  for i = 1, c * r do
    local s = 0
    for _, j in ipairs(neighbours(i, c, r)) do s = s + m[j] end
    out[i] = s
  end
  return out
end

-- Opens cell `i` of `st`, and around every cell with no mine near, the cells around it (a stack
-- instead of recursion). Whether it was a mine. Flagged cells stay shut.
function open(st, m, cn, c, r, i)
  if st[i] ~= 0 then return false end
  if m[i] == 1 then
    st[i] = 1
    return true
  end
  local stack = {i}
  st[i] = 1
  while #stack > 0 do
    local j = table.remove(stack)
    if cn[j] == 0 then
      for _, k in ipairs(neighbours(j, c, r)) do
        if st[k] == 0 and m[k] == 0 then
          st[k] = 1
          stack[#stack + 1] = k
        end
      end
    end
  end
  return false
end

-- Whether every cell without a mine is open.
function cleared(st, m)
  for i = 1, #st do
    if m[i] == 0 and st[i] ~= 1 then return false end
  end
  return true
end

local function flags()
  local f = 0
  for i = 1, cols * rows do
    if state[i] == 2 then f = f + 1 end
  end
  return f
end

local function keep()
  local data = {level = level, best = best}
  if placed and not over then data.game = {mines = mines, state = state, time = elapsed} end
  save(data)
end

local function setup(k)
  level = k
  cols, rows, n = LEVELS[k][1], LEVELS[k][2], LEVELS[k][3]
  input("grid", {cols = cols, rows = rows})
end

local function start()
  mines, state, counts = {}, {}, {}
  for i = 1, cols * rows do
    mines[i] = 0
    state[i] = 0
    counts[i] = 0
  end
  placed, over, boom = false, nil, nil
  base, mark, elapsed = 0, last, 0
  timer(0)
end

local function finish(result)
  over = result
  timer(0)
  if result == "won" then
    -- The mines left are flagged.
    for i = 1, cols * rows do
      if mines[i] == 1 then state[i] = 2 end
    end
    if best[level] == 0 or elapsed < best[level] then best[level] = elapsed end
  end
end

-- Opens cell `i`; the first opening lays the mines.
local function reveal(i)
  if not placed then
    mines = place(cols, rows, n, i)
    counts = count(mines, cols, rows)
    placed = true
    base, mark, elapsed = 0, last, 0
    timer(1000)
  end
  if open(state, mines, counts, cols, rows, i) then
    boom = i
    finish("lost")
  elseif cleared(state, mines) then
    finish("won")
  end
end

-- OK on an open number whose flags are all set opens the cells around it.
local function chord(i)
  local around = neighbours(i, cols, rows)
  local f = 0
  for _, j in ipairs(around) do
    if state[j] == 2 then f = f + 1 end
  end
  if f ~= counts[i] then return end
  for _, j in ipairs(around) do
    if not over and state[j] == 0 then reveal(j) end
  end
end

local function toggle_flag(i)
  if over or state[i] == 1 then return end
  state[i] = state[i] == 2 and 0 or 2
end

local function new_game(k)
  if placed and not over and not confirm(de() and "Neues Spiel?" or "New game?") then return end
  setup(k)
  start()
  keep()
  labels()
end

local function list_of(v, size)
  if type(v) ~= "table" or #v ~= size then return false end
  for _, x in ipairs(v) do
    if math.type(x) ~= "integer" then return false end
  end
  return true
end

function init()
  last = 0
  local saved = load()
  if type(saved) ~= "table" then saved = {} end
  best = list_of(saved.best, 3) and saved.best or {0, 0, 0}
  flagging = false
  setup((saved.level == 2 or saved.level == 3) and saved.level or 1)
  start()
  local game = saved.game
  if type(game) == "table" and list_of(game.mines, cols * rows) and list_of(game.state, cols * rows) then
    mines, state = game.mines, game.state
    counts = count(mines, cols, rows)
    placed = true
    elapsed = math.type(game.time) == "integer" and game.time or 0
    base, mark = elapsed, 0
    timer(1000)
  end
  labels()
end

function tick(ms)
  last = ms
  if placed and not over then elapsed = base + (ms - mark) // 1000 end
end

function pick(col, row)
  if over then
    new_game(level)
    return
  end
  local i = (row - 1) * cols + col
  if state[i] == 1 then
    chord(i)
  elseif flagging then
    toggle_flag(i)
  elseif state[i] == 0 then
    reveal(i)
  end
  keep()
end

function key(k)
  if k == "soft1" then
    new_game(level)
  elseif k == "soft2" then
    new_game(level % 3 + 1)
  elseif k == "soft3" then
    flagging = not flagging
    labels()
  elseif k == "0" or k == "." then
    local c = cursor()
    toggle_flag((c.row - 1) * cols + c.col)
    keep()
  end
end

function leave() keep() end

local function clock(s)
  if s >= 3600 then return string.format("%d:%02d:%02d", s // 3600, s // 60 % 60, s % 60) end
  return string.format("%d:%02d", s // 60, s % 60)
end

local function side_panel()
  local s = side()
  local room = s.w - 8
  local x, y = s.x + 4, s.y + 4
  local small = text_small_height() + 4
  local left = (de() and "Minen " or "Mines ") .. (n - flags())
  local status
  if over == "won" then
    status = {de() and "Gewonnen!" or "You won!", "amber"}
  elseif over == "lost" then
    status = {de() and "Verloren" or "You lost", "error"}
  elseif flagging then
    status = {de() and "OK setzt Flagge" or "OK sets a flag", "muted"}
  else
    status = {de() and "0 setzt Flagge" or "0 sets a flag", "muted"}
  end
  local lines = {{level_name(level), "muted"}, {left, "text", true}, {clock(elapsed), "text", true}}
  if best[level] > 0 then lines[#lines + 1] = {(de() and "Bestzeit " or "Best ") .. clock(best[level]), "muted"} end
  lines[#lines + 1] = status
  if s.h >= #lines * small + 2 * text_height() then
    for _, l in ipairs(lines) do
      if l[3] and text_width(l[1]) <= room then
        text(x, y, l[1], l[2])
        y = y + text_height() + 4
      elseif text_small_width(l[1]) <= room and y + small <= s.y + s.h then
        text_small(x, y, l[1], l[2])
        y = y + small
      end
    end
  else
    local line = left .. "   " .. clock(elapsed)
    if over then line = line .. "   " .. status[1] end
    if text_small_width(line) <= room and y + small <= s.y + s.h then text_small(x, y, line, over and status[2] or "text") end
  end
end

local NUMBER = {"cyan", "result", "error", "amber", "text", "text", "text", "text"}

local function flag_at(x, y, w, h)
  local px = x + w * 2 // 5
  line(px, y + h // 5, px, y + h * 4 // 5, "text", 2)
  rect(px, y + h // 5, math.max(w // 3, 2), math.max(h // 4, 2), "error")
end

function draw()
  clear("background")
  local g = grid(cols, rows)
  rect(g.x, g.y, g.w, g.h, "hairline")
  local big = text_height() + 2
  for i = 1, cols * rows do
    local c = g:cell((i - 1) % cols + 1, (i - 1) // cols + 1)
    local x, y, w, h = c.x + 1, c.y + 1, c.w - 2, c.h - 2
    local st = state[i]
    if st == 1 then
      rect(x, y, w, h, i == boom and "error" or "background")
      if mines[i] == 1 then
        circle(x + w // 4, y + h // 4, math.max(w // 2, 2), "text")
      elseif counts[i] > 0 then
        local s = tostring(counts[i])
        if big <= h then
          text(x + (w - text_width(s)) // 2, y + (h - text_height()) // 2, s, NUMBER[counts[i]])
        else
          text_small(x + (w - text_small_width(s)) // 2, y + (h - text_small_height()) // 2, s, NUMBER[counts[i]])
        end
      end
    else
      round(x, y, w, h, 3, "faint")
      if st == 2 then
        flag_at(x, y, w, h)
        if over == "lost" and mines[i] == 0 then
          -- A flag on a cell without a mine.
          line(x + 2, y + 2, x + w - 3, y + h - 3, "error", 2)
          line(x + w - 3, y + 2, x + 2, y + h - 3, "error", 2)
        end
      elseif over == "lost" and mines[i] == 1 then
        circle(x + w // 4, y + h // 4, math.max(w // 2, 2), "muted")
      end
    end
  end
  side_panel()
end
