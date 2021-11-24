use rapier2d::prelude::*;
use std::collections::HashMap;

pub trait IWorld {
    fn add_player(&mut self, uid: String) -> RigidBodyHandle;
    fn get_body_set(&mut self) -> &mut RigidBodySet;
    fn get_collider_set(&mut self) -> &mut ColliderSet;
    fn get_body_and_collider_set(&mut self) -> (&mut RigidBodySet, &mut ColliderSet);
    fn get_physics_pipeline(&mut self) -> &PhysicsPipeline;
    fn step(&mut self);
    fn start_simulation(&mut self);
}

pub struct World {
    pub body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub players: HashMap<String, RigidBodyHandle>,
    pub ball: RigidBodyHandle,
    pub physics_pipeline: PhysicsPipeline,
}

impl IWorld for World {
    fn get_body_and_collider_set(&mut self) -> (&mut RigidBodySet, &mut ColliderSet) {
        (&mut self.body_set, &mut self.collider_set)
    }

    fn get_body_set(&mut self) -> &mut RigidBodySet {
        &mut self.body_set
    }

    fn get_collider_set(&mut self) -> &mut ColliderSet {
        &mut self.collider_set
    }

    fn get_physics_pipeline(&mut self) -> &PhysicsPipeline {
        &mut self.physics_pipeline
    }

    fn add_player(&mut self, uid: String) -> RigidBodyHandle {
        let rad = 10.0;
        let player_body = RigidBodyBuilder::new_dynamic()
            .translation(vector![0.0, 0.0])
            .gravity_scale(0.0)
            .linear_damping(0.5)
            .angular_damping(1.0)
            .build();
        let player_handle = self.body_set.insert(player_body);
        let collider = ColliderBuilder::ball(rad).restitution(1.).build();
        self.collider_set
            .insert_with_parent(collider, player_handle, &mut self.body_set);
        self.players.insert(uid, player_handle);
        player_handle
    }

    fn start_simulation(&mut self) {
        use std::{thread, time};

        let delay = time::Duration::from_millis(50);

        let gravity = vector![0.0, -9.81];
        let integration_parameters = IntegrationParameters::default();
        let mut physics_pipeline = PhysicsPipeline::new();
        let mut island_manager = IslandManager::new();
        let mut broad_phase = BroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut joint_set = JointSet::new();
        let mut ccd_solver = CCDSolver::new();
        let physics_hooks = ();
        let event_handler = ();

        loop {
            physics_pipeline.step(
                &gravity,
                &integration_parameters,
                &mut island_manager,
                &mut broad_phase,
                &mut narrow_phase,
                &mut self.body_set,
                &mut self.collider_set,
                &mut joint_set,
                &mut ccd_solver,
                &physics_hooks,
                &event_handler,
            );

            thread::sleep(delay);

            // println!("ball: {:?}", self.body_set[self.ball].translation());
                // println!("player: {:?}", self.body_set[*player].translation());
        }
    }

    fn step(&mut self) {
        let gravity = vector![0.0, -9.81];
        let integration_parameters = IntegrationParameters::default();
        let mut island_manager = IslandManager::new();
        let mut broad_phase = BroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut joint_set = JointSet::new();
        let mut ccd_solver = CCDSolver::new();
        let physics_hooks = ();
        let event_handler = ();

        let ball = &mut self.body_set[self.ball];
        ball.apply_impulse(vector![100000.0, 0.0], true);

        self.physics_pipeline.step(
            &gravity,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut self.body_set,
            &mut self.collider_set,
            &mut joint_set,
            &mut ccd_solver,
            &physics_hooks,
            &event_handler,
        );

        println!("ball: {:?}", self.body_set[self.ball].translation());

        if let Some(player) = self.players.get("127.0.0.1:12355") {
            println!("ball: {:?}", self.body_set[self.ball].translation());
            println!("player: {:?}", self.body_set[*player].translation());
        }

        // let mut rigid_body_set = RigidBodySet::new();
        // let mut collider_set = ColliderSet::new();

        // /* Create the ground. */
        // let collider = ColliderBuilder::cuboid(100.0, 0.1).build();
        // collider_set.insert(collider);

        // /* Create the bouncing ball. */
        // let rigid_body = RigidBodyBuilder::new_dynamic()
        //     .translation(vector![0.0, 10.0])
        //     .build();
        // let collider = ColliderBuilder::ball(0.5).restitution(0.7).build();
        // let ball_body_handle = rigid_body_set.insert(rigid_body);
        // collider_set.insert_with_parent(collider, ball_body_handle, &mut rigid_body_set);

        // /* Create other structures necessary for the simulation. */
        // let gravity = vector![0.0, -9.81];
        // let integration_parameters = IntegrationParameters::default();
        // let mut island_manager = IslandManager::new();
        // let mut broad_phase = BroadPhase::new();
        // let mut narrow_phase = NarrowPhase::new();
        // let mut joint_set = JointSet::new();
        // let mut ccd_solver = CCDSolver::new();
        // let physics_hooks = ();
        // let event_handler = ();

        // /* Run the game loop, stepping the simulation once per frame. */
        // for _ in 0..200 {
        //     self.physics_pipeline.step(
        //         &gravity,
        //         &integration_parameters,
        //         &mut island_manager,
        //         &mut broad_phase,
        //         &mut narrow_phase,
        //         &mut self.,
        //         &mut collider_set,
        //         &mut joint_set,
        //         &mut ccd_solver,
        //         &physics_hooks,
        //         &event_handler,
        //     );
    
        //     let ball_body = &rigid_body_set[ball_body_handle];
        //     println!("Ball altitude: {}", ball_body.translation().y);
        // }
        
    }
}

pub fn init_world() -> World {
    /*
     * World
     */
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();

    /*
     * Ground.
     */
    let ground_size = 5.0;
    let ground_area = vector![900.0, 500.0];

    /*
     * Frame.
     */
    let rigid_body = RigidBodyBuilder::new_static()
        .translation(vector![-ground_area.x / 2.0, 0.0])
        .build();
    let handle = bodies.insert(rigid_body);
    let collider = ColliderBuilder::cuboid(ground_size, ground_area.y / 2.0).build();
    colliders.insert_with_parent(collider, handle, &mut bodies);

    let rigid_body = RigidBodyBuilder::new_static()
        .translation(vector![ground_area.x / 2.0, 0.0])
        .build();
    let handle = bodies.insert(rigid_body);
    let collider = ColliderBuilder::cuboid(ground_size, ground_area.y / 2.0).build();
    colliders.insert_with_parent(collider, handle, &mut bodies);

    let rigid_body = RigidBodyBuilder::new_static()
        .translation(vector![0.0, -ground_area.y / 2.0])
        .build();
    let handle = bodies.insert(rigid_body);
    let collider = ColliderBuilder::cuboid(ground_area.x / 2.0, ground_size).build();
    colliders.insert_with_parent(collider, handle, &mut bodies);

    let rigid_body = RigidBodyBuilder::new_static()
        .translation(vector![0.0, ground_area.y / 2.0])
        .build();
    let handle = bodies.insert(rigid_body);
    let collider = ColliderBuilder::cuboid(ground_area.x / 2.0, ground_size).build();
    colliders.insert_with_parent(collider, handle, &mut bodies);

    // Build ball
    let rad = 8.0;
    let ball_body = RigidBodyBuilder::new_dynamic()
        .translation(vector![250.0, 250.0])
        .gravity_scale(1.0)
        .linear_damping(0.5)
        .angular_damping(1.0)
        .build();
    let ball_handle = bodies.insert(ball_body);
    let collider = ColliderBuilder::ball(rad).restitution(1.).build();
    colliders.insert_with_parent(collider, ball_handle, &mut bodies);

    World {
        body_set: bodies,
        collider_set: colliders,
        players: HashMap::new(),
        ball: ball_handle,
        physics_pipeline: PhysicsPipeline::new()
    }
}
