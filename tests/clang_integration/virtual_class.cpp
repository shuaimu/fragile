// Virtual function test file for Fragile Clang integration
// Tests: vtable generation, virtual method detection, constructor vtable init

// Base class with virtual function
class Animal {
public:
    virtual void speak() {}
    virtual int legs() { return 0; }
};

// Derived class overriding virtual functions
class Dog : public Animal {
public:
    void speak() override {}
    int legs() override { return 4; }
};

// Class with pure virtual function (abstract)
class Shape {
public:
    virtual double area() = 0;  // pure virtual
    virtual void draw() {}
};

// Concrete class implementing pure virtual
class Circle : public Shape {
public:
    Circle(double r) : radius(r) {}
    double area() override { return 3.14159 * radius * radius; }

private:
    double radius;
};

// Non-polymorphic class (no virtual functions)
class Point {
public:
    Point(int x, int y) : x_(x), y_(y) {}
    int x() const { return x_; }
    int y() const { return y_; }
private:
    int x_;
    int y_;
};
