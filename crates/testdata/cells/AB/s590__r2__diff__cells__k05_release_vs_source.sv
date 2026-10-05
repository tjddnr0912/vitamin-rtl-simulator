module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = v;
  endfunction
  wire m, x, y;
  tri w;
  assign x = f(m, 3);
  assign w = a ? f(m, 4) : 1'bz;
  assign y = f(a, 2);
  assign w = a ? 1'bz : f(a, 5);
  assign m = f(a, 1);
  always @(x) $display("ev x=%b t=%0t", x, $time);
  always @(y) $display("ev y=%b t=%0t", y, $time);
  always @(w) $display("ev w=%b t=%0t", w, $time);
  always @(x or y) $display("ev xy x=%b y=%b t=%0t", x, y, $time);
  initial begin a = 1; #1 $display("t1 m=%b x=%b y=%b w=%b", m, x, y, w); $finish; end
  initial #10 $finish;
endmodule
