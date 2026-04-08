// Matrix multiplication experiment - party 1

#include "matmul_config.h"

#include "share/Spdz2kShare.h"
#include "protocols/Circuit.h"
#include "utils/print_vector.h"

using namespace std;
using namespace md_ml;
using namespace md_ml::experiments::matmul;

int main() {
    using ShrType = Spdz2kShare64;

    PartyWithFakeOffline<ShrType> party(1, 2, 6060, kJobName);
    Circuit<ShrType> circuit(party);

    auto a = circuit.input(0, dim, dim);
    auto b = circuit.input(0, dim, dim);
    auto c = circuit.multiply(a, b);
    auto d = circuit.output(c);
    circuit.addEndpoint(d);

    circuit.readOfflineFromFile();
    circuit.runOnlineWithBenckmark();

    circuit.printStats();

    return 0;
}
