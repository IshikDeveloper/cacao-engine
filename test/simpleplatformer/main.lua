-- Example Game: Simple Platformer
-- This demonstrates the Cacao Engine API usage

-- Game state
local player = {
    x = 400,
    y = 300,
    width = 32,
    height = 32,
    vel_x = 0,
    vel_y = 0,
    speed = 200,
    jump_power = 400,
    on_ground = false,
    sprite = nil
}

local world = {
    gravity = 1200,
    platforms = {},
    collectibles = {},
    background_color = {0.2, 0.4, 0.8, 1.0}
}

local game_state = {
    score = 0,
    lives = 3,
    paused = false,
    level = 1
}

local sounds = {}

-- Initialize the game
function init()
    print("Initializing Simple Platformer...")
    
    -- Load player sprite
    player.sprite = cacao.assets.load_sprite("player.png")
    
    -- Load sounds
    sounds.jump = cacao.assets.load_sound("jump.wav")
    sounds.collect = cacao.assets.load_sound("collect.wav")
    sounds.background = cacao.assets.load_sound("background.ogg")
    
    -- Load save data
    game_state.score = cacao.saves.read("high_score", 0)
    game_state.level = cacao.saves.read("current_level", 1)
    
    -- Create some platforms
    table.insert(world.platforms, {x = 200, y = 400, width = 200, height = 20})
    table.insert(world.platforms, {x = 500, y = 300, width = 150, height = 20})
    table.insert(world.platforms, {x = 100, y = 500, width = 600, height = 20})
    
    -- Create collectibles
    for i = 1, 5 do
        table.insert(world.collectibles, {
            x = 100 + i * 120,
            y = 200,
            width = 16,
            height = 16,
            collected = false
        })
    end
    
    -- Start background music
    cacao.audio.play_music(sounds.background, true)
    
    -- Setup input mappings
    setup_input()
end

function setup_input()
    -- Input is already mapped by the engine, but we can add custom mappings
    cacao.input.map("restart", {"r", "gamepad_start"})
    cacao.input.map("pause", {"p", "escape", "gamepad_select"})
end

-- Update game logic
function update(delta_time)
    if game_state.paused then
        handle_pause_input()
        return
    end
    
    handle_input(delta_time)
    update_physics(delta_time)
    update_collectibles()
    check_win_condition()
end

function handle_input(delta_time)
    local input_x = 0
    
    -- Movement input
    if cacao.input.is_action_pressed("move_left") then
        input_x = input_x - 1
    end
    
    if cacao.input.is_action_pressed("move_right") then
        input_x = input_x + 1
    end
    
    -- Apply movement
    player.vel_x = input_x * player.speed
    
    -- Jump input
    if cacao.input.is_action_just_pressed("jump") and player.on_ground then
        player.vel_y = -player.jump_power
        player.on_ground = false
        cacao.audio.play_sound(sounds.jump, false)
    end
    
    -- Pause
    if cacao.input.is_action_just_pressed("pause") then
        game_state.paused = not game_state.paused
    end
    
    -- Restart
    if cacao.input.is_action_just_pressed("restart") then
        restart_level()
    end
end

function handle_pause_input()
    if cacao.input.is_action_just_pressed("pause") or 
       cacao.input.is_action_just_pressed("action") then
        game_state.paused = false
    end
    
    if cacao.input.is_action_just_pressed("restart") then
        restart_level()
        game_state.paused = false
    end
end

function update_physics(delta_time)
    -- Apply gravity
    if not player.on_ground then
        player.vel_y = player.vel_y + world.gravity * delta_time
    end
    
    -- Update position
    player.x = player.x + player.vel_x * delta_time
    player.y = player.y + player.vel_y * delta_time
    
    -- Keep player in bounds
    if player.x < 0 then
        player.x = 0
        player.vel_x = 0
    elseif player.x + player.width > 800 then
        player.x = 800 - player.width
        player.vel_x = 0
    end
    
    -- Platform collision
    player.on_ground = false
    for _, platform in ipairs(world.platforms) do
        if check_collision(player, platform) then
            -- Landing on top of platform
            if player.vel_y > 0 and player.y < platform.y then
                player.y = platform.y - player.height
                player.vel_y = 0
                player.on_ground = true
            end
        end
    end
    
    -- Death condition (fell off screen)
    if player.y > 600 then
        respawn_player()
    end
end

function update_collectibles()
    for _, collectible in ipairs(world.collectibles) do
        if not collectible.collected and check_collision(player, collectible) then
            collectible.collected = true
            game_state.score = game_state.score + 100
            cacao.audio.play_sound(sounds.collect, false)
            
            -- Save high score
            local high_score = cacao.saves.read("high_score", 0)
            if game_state.score > high_score then
                cacao.saves.write("high_score", game_state.score)
                cacao.saves.save_to_disk()
            end
        end
    end
end

function check_collision(rect1, rect2)
    return rect1.x < rect2.x + rect2.width and
           rect1.x + rect1.width > rect2.x and
           rect1.y < rect2.y + rect2.height and
           rect1.y + rect1.height > rect2.y
end

function check_win_condition()
    local all_collected = true
    for _, collectible in ipairs(world.collectibles) do
        if not collectible.collected then
            all_collected = false
            break
        end
    end
    
    if all_collected then
        next_level()
    end
end

function next_level()
    game_state.level = game_state.level + 1
    cacao.saves.write("current_level", game_state.level)
    cacao.saves.save_to_disk()
    
    -- Reset collectibles
    for _, collectible in ipairs(world.collectibles) do
        collectible.collected = false
    end
    
    -- Reset player position
    player.x = 400
    player.y = 200
    player.vel_x = 0
    player.vel_y = 0
end

function restart_level()
    -- Reset player
    player.x = 400
    player.y = 300
    player.vel_x = 0
    player.vel_y = 0
    player.on_ground = false
    
    -- Reset collectibles
    for _, collectible in ipairs(world.collectibles) do
        collectible.collected = false
    end
    
    -- Reduce lives
    game_state.lives = game_state.lives - 1
    if game_state.lives <= 0 then
        game_over()
    end
end

function respawn_player()
    restart_level()
end

function game_over()
    game_state.lives = 3
    game_state.level = 1
    game_state.score = 0
    
    -- Reset to level 1
    cacao.saves.write("current_level", 1)
    cacao.saves.save_to_disk()
end

-- Render the game
function render()
    -- Clear screen with background color
    cacao.renderer.clear(world.background_color)
    
    -- Set camera to follow player
    local camera_x = player.x - 400  -- Center on player
    local camera_y = player.y - 300
    cacao.renderer.set_camera(camera_x, camera_y, 1.0)
    
    -- Draw platforms
    for _, platform in ipairs(world.platforms) do
        cacao.renderer.draw_rect(
            platform.x, platform.y, 
            platform.width, platform.height, 
            {0.4, 0.4, 0.4, 1.0}
        )
    end
    
    -- Draw collectibles
    for _, collectible in ipairs(world.collectibles) do
        if not collectible.collected then
            cacao.renderer.draw_rect(
                collectible.x, collectible.y,
                collectible.width, collectible.height,
                {1.0, 1.0, 0.0, 1.0}  -- Yellow
            )
        end
    end
    
    -- Draw player
    if player.sprite then
        cacao.renderer.draw_sprite(
            player.sprite,
            player.x + player.width / 2,  -- Center the sprite
            player.y + player.height / 2,
            0,  -- rotation
            1.0  -- scale
        )
    else
        -- Fallback rectangle if sprite not loaded
        cacao.renderer.draw_rect(
            player.x, player.y,
            player.width, player.height,
            {0.0, 1.0, 0.0, 1.0}  -- Green
        )
    end
    
    -- Draw UI
    draw_ui()
    
    -- Draw pause overlay
    if game_state.paused then
        draw_pause_overlay()
    end
end

function draw_ui()
    -- Reset camera for UI
    cacao.renderer.set_camera(0, 0, 1.0)
    
    -- Draw score
    cacao.renderer.draw_text(
        "Score: " .. game_state.score,
        10, 10, 24,
        {1.0, 1.0, 1.0, 1.0}
    )
    
    -- Draw lives
    cacao.renderer.draw_text(
        "Lives: " .. game_state.lives,
        10, 40, 24,
        {1.0, 1.0, 1.0, 1.0}
    )
    
    -- Draw level
    cacao.renderer.draw_text(
        "Level: " .. game_state.level,
        10, 70, 24,
        {1.0, 1.0, 1.0, 1.0}
    )
    
    -- Draw high score
    local high_score = cacao.saves.read("high_score", 0)
    cacao.renderer.draw_text(
        "High Score: " .. high_score,
        600, 10, 20,
        {0.8, 0.8, 0.8, 1.0}
    )
end

function draw_pause_overlay()
    -- Semi-transparent overlay
    cacao.renderer.draw_rect(0, 0, 800, 600, {0.0, 0.0, 0.0, 0.7})
    
    -- Pause text
    cacao.renderer.draw_text(
        "PAUSED",
        350, 250, 48,
        {1.0, 1.0, 1.0, 1.0}
    )
    
    cacao.renderer.draw_text(
        "Press P or ESC to continue",
        280, 300, 24,
        {0.8, 0.8, 0.8, 1.0}
    )
    
    cacao.renderer.draw_text(
        "Press R to restart level",
        300, 330, 24,
        {0.8, 0.8, 0.8, 1.0}
    )
end

-- Cleanup when game shuts down
function cleanup()
    print("Cleaning up Simple Platformer...")
    
    -- Save final state
    cacao.saves.write("high_score", game_state.score)
    cacao.saves.write("current_level", game_state.level)
    cacao.saves.save_to_disk()
    
    -- Stop all audio
    cacao.audio.stop_all()
end

-- Debug function (can be called from console)
function debug_info()
    print("=== Debug Info ===")
    print("Player position: " .. player.x .. ", " .. player.y)
    print("Player velocity: " .. player.vel_x .. ", " .. player.vel_y)
    print("On ground: " .. tostring(player.on_ground))
    print("Score: " .. game_state.score)
    print("Lives: " .. game_state.lives)
    print("Level: " .. game_state.level)
    
    local save_stats = cacao.saves.get_stats()
    print("Save data keys: " .. save_stats.total_keys)
    print("Save data size: " .. save_stats.estimated_size .. " bytes")
end