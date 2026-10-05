`timescale 1ns/1ns
class C;
  int a, b, c;
  function void run();
    for (int i = 0; i < 200; i++) begin
      case ($urandom_range(0, 1)) inside 0: a = a + 1; 1: b = b + 1; default: c = c + 1; endcase
    end
  endfunction
  function void run2();
    for (int i = 0; i < 200; i++) begin
      case ($random & 1) inside 0: a = a + 1; 1: b = b + 1; default: c = c + 1; endcase
    end
  endfunction
endclass
module top;
  C o;
  initial begin
    o = new;
    o.run();
    $display("ci urandom_range a+b=%0d c=%0d", o.a + o.b, o.c);
    o.a = 0; o.b = 0; o.c = 0;
    o.run2();
    $display("ci random a+b=%0d c=%0d", o.a + o.b, o.c);
    $finish;
  end
  initial #1000 $finish;
endmodule
