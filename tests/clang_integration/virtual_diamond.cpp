// Virtual diamond inheritance test for Fragile
// Ensures virtual base is shared and accessible through derived classes

class A {
public:
    int a;
    A(int v) : a(v) {}
    int getA() { return a; }
};

class B : virtual public A {
public:
    int b;
    B(int v) : A(v), b(v + 1) {}
    int getAFromB() { return a; }
};

class C : virtual public A {
public:
    int c;
    C(int v) : A(v), c(v + 2) {}
    int getAFromC() { return a; }
};

class D : public B, public C {
public:
    int d;
    D(int v) : A(v), B(v), C(v), d(v + 3) {}
    int sum() { return a + b + c + d; }
    int sumViaBases() { return B::getAFromB() + C::getAFromC() + d; }
};

int diamond_sum(int v) {
    D d(v);
    return d.sum();
}

int diamond_sum_via_bases(int v) {
    D d(v);
    return d.sumViaBases();
}
