module top;
  logic a, b;
  wire y, d, m;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign y = f(a);
  assign #1 d = b;
  assign m = a;
  assign m = b;
  initial begin
    $display("i0 d=%b m=%b y=%b", d, m, y);
    a = 1'b0; b = 1'b1;
    #0 $display("i0b d=%b m=%b y=%b", d, m, y);
  end
  initial #1 $display("t1 d=%b m=%b y=%b", d, m, y);
  initial #10 $finish;
endmodule
