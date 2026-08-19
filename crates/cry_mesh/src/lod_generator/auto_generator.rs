use super::auto_lod_settings::AutoLodSettings;
use super::lod_builder::LODMeshBuilder;
use super::types::LODGenParams;
use super::visual_change::VisualChangeCalculator;
use crate::mesh::CNodeCGF;
use cry_core::math::Matrix34;

pub struct AutoGenerator {
    pub settings: AutoLodSettings,
    pub params: LODGenParams,
}

impl AutoGenerator {
    pub fn new(settings: AutoLodSettings) -> Self {
        Self {
            settings,
            params: LODGenParams::default(),
        }
    }

    pub fn generate_lods_for_nodes(&self, nodes: &mut Vec<CNodeCGF>) {
        let mut new_lod_nodes = Vec::new();

        for node in nodes.iter() {
            let param = self.settings.get_node_param(&node.name);
            if !param.auto_generate || node.mesh.positions.is_empty() {
                continue;
            }

            let mut calc = VisualChangeCalculator::new(self.params.clone());
            calc.load_mesh(&node.mesh.positions, &node.mesh.indices);
            let seq = calc.process();

            let mut current_percent = param.percent;
            for lod_lvl in 1..=param.lod_count {
                let lod_mesh = LODMeshBuilder::build_lod_mesh(&seq, current_percent * 100.0);
                let lod_name = format!("$lod{}_{}_0_", lod_lvl, node.name);

                new_lod_nodes.push(CNodeCGF {
                    name: lod_name,
                    mesh: lod_mesh,
                    world_tm: Matrix34::IDENTITY,
                    local_tm: Matrix34::IDENTITY,
                    is_identity_matrix: true,
                    is_physics_proxy: false,
                    properties: String::new(),
                });

                current_percent *= param.percent;
            }
        }

        nodes.extend(new_lod_nodes);
    }
}
