// Copyright 2019-2020 Crustio Technologies Ltd.
// This file is part of Substrate.

// You should have received a copy of the GNU General Public License
// along with Substrate. If not, see <http://www.gnu.org/licenses/>.

use wasm_builder_runner::WasmBuilder;

fn main() {
    WasmBuilder::new()
        .with_current_project()
        .with_wasm_builder_from_crates("2.0.0")
        .export_heap_base()
        .import_memory()
        .build()
}

