// C++ party 0 for cross-language tests with Rust party 1.
// Usage: ./cross_lang_party_0 <test_name>
// Tests: multiply_trunc, multiply, add, gtz, matmul

#include <iostream>
#include <iomanip>
#include <string>
#include <vector>
#include <cstdlib>

#include "share/Spdz2kShare.h"
#include "protocols/Circuit.h"
#include "utils/fixed_point.h"
#include "utils/print_vector.h"

using namespace std;
using namespace md_ml;

using ShrType = Spdz2kShare64;
using ClearType = ShrType::ClearType;

static constexpr size_t PORT = 7070;

void test_multiply_trunc() {
    PartyWithFakeOffline<ShrType> party(0, 2, PORT, "xtest_mt_cpp");
    Circuit<ShrType> circuit(party);

    auto a = circuit.input(0, 1, 1);
    auto b = circuit.input(0, 1, 1);
    auto c = circuit.multiplyTrunc(a, b);
    auto d = circuit.output(c);
    circuit.addEndpoint(d);

    vector<ClearType> vec{double2fix<ClearType>(1.5)};
    a->setInput(vec);
    b->setInput(vec);

    circuit.readOfflineFromFile();
    circuit.runOnline();

    auto o = d->getClear();
    cout << fixed << setprecision(6);
    cout << "RESULT:" << fix2double<ClearType>(o[0]) << endl;
}

void test_multiply() {
    PartyWithFakeOffline<ShrType> party(0, 2, PORT, "xtest_mul_cpp");
    Circuit<ShrType> circuit(party);

    auto a = circuit.input(0, 1, 1);
    auto b = circuit.input(0, 1, 1);
    auto c = circuit.multiply(a, b);
    auto d = circuit.output(c);
    circuit.addEndpoint(d);

    vector<ClearType> va{3};
    vector<ClearType> vb{5};
    a->setInput(va);
    b->setInput(vb);

    circuit.readOfflineFromFile();
    circuit.runOnline();

    auto o = d->getClear();
    cout << "RESULT:" << o[0] << endl;
}

void test_add() {
    PartyWithFakeOffline<ShrType> party(0, 2, PORT, "xtest_add_cpp");
    Circuit<ShrType> circuit(party);

    auto a = circuit.input(0, 1, 1);
    auto b = circuit.input(0, 1, 1);
    auto c = circuit.add(a, b);
    auto d = circuit.output(c);
    circuit.addEndpoint(d);

    vector<ClearType> va{7};
    vector<ClearType> vb{8};
    a->setInput(va);
    b->setInput(vb);

    circuit.readOfflineFromFile();
    circuit.runOnline();

    auto o = d->getClear();
    cout << "RESULT:" << o[0] << endl;
}

void test_gtz() {
    PartyWithFakeOffline<ShrType> party(0, 2, PORT, "xtest_gtz_cpp");
    Circuit<ShrType> circuit(party);

    auto input_x = circuit.input(0, 10, 1);
    auto g = circuit.gtz(input_x);
    auto d = circuit.output(g);
    circuit.addEndpoint(d);

    // Values: -5, -4, -3, -2, -1, 0, 1, 2, 3, 4
    // Two's complement: large unsigned values for negatives
    vector<ClearType> vec;
    for (int i = -5; i < 5; i++) {
        vec.push_back(static_cast<ClearType>(i));
    }
    input_x->setInput(vec);

    circuit.readOfflineFromFile();
    circuit.runOnline();

    auto o = d->getClear();
    cout << "RESULT:";
    for (size_t i = 0; i < o.size(); i++) {
        if (i > 0) cout << ",";
        cout << o[i];
    }
    cout << endl;
}

void test_matmul() {
    const size_t dim = 4;
    PartyWithFakeOffline<ShrType> party(0, 2, PORT, "xtest_mm_cpp");
    Circuit<ShrType> circuit(party);

    auto a = circuit.input(0, dim, dim);
    auto b = circuit.input(0, dim, dim);
    auto c = circuit.multiplyTrunc(a, b);
    auto d = circuit.output(c);
    circuit.addEndpoint(d);

    // Use simple fixed-point values: identity-like matrix * constant matrix
    vector<ClearType> va(dim * dim, 0);
    vector<ClearType> vb(dim * dim, 0);
    for (size_t i = 0; i < dim; i++) {
        for (size_t j = 0; j < dim; j++) {
            va[i * dim + j] = double2fix<ClearType>(static_cast<double>(i * dim + j + 1) * 0.1);
            vb[i * dim + j] = double2fix<ClearType>(static_cast<double>(j * dim + i + 1) * 0.1);
        }
    }
    a->setInput(va);
    b->setInput(vb);

    circuit.readOfflineFromFile();
    circuit.runOnline();

    auto o = d->getClear();
    cout << fixed << setprecision(6);
    cout << "RESULT:";
    for (size_t i = 0; i < o.size(); i++) {
        if (i > 0) cout << ",";
        cout << fix2double<ClearType>(o[i]);
    }
    cout << endl;
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        cerr << "Usage: " << argv[0] << " <test_name>" << endl;
        cerr << "Tests: multiply_trunc, multiply, add, gtz, matmul" << endl;
        return 1;
    }

    string test_name = argv[1];

    if (test_name == "multiply_trunc") test_multiply_trunc();
    else if (test_name == "multiply") test_multiply();
    else if (test_name == "add") test_add();
    else if (test_name == "gtz") test_gtz();
    else if (test_name == "matmul") test_matmul();
    else {
        cerr << "Unknown test: " << test_name << endl;
        return 1;
    }

    return 0;
}
