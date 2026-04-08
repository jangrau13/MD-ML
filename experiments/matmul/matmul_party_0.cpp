// Matrix multiplication experiment - party 0

#include "matmul_config.h"

#include "share/Spdz2kShare.h"
#include "protocols/Circuit.h"
#include "utils/print_vector.h"

using namespace std;
using namespace md_ml;
using namespace md_ml::experiments::matmul;

int main() {
    using ShrType = Spdz2kShare64;
    using ClearType = ShrType::ClearType;

    // Fill with 1s (dim x dim identity-like test)
    vector<ClearType> mat(dim * dim, 1);

    PartyWithFakeOffline<ShrType> party(0, 2, 6060, kJobName);
    Circuit<ShrType> circuit(party);

    auto a = circuit.input(0, dim, dim);
    auto b = circuit.input(0, dim, dim);
    auto c = circuit.multiply(a, b);
    auto d = circuit.output(c);
    circuit.addEndpoint(d);

    a->setInput(mat);
    b->setInput(mat);

    circuit.readOfflineFromFile();
    circuit.runOnlineWithBenckmark();

    circuit.printStats();

    return 0;
}
