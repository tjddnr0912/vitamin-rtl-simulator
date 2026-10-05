module top;
  logic a;
  function automatic logic f(input logic x);
    $display("f x=%b t=%0t", x, $time);
    f = ~x;
  endfunction
  wire [4:0] c;
  assign c[0] = f(a);
  genvar k;
  for (k = 0; k < 4; k++) begin : g
    assign c[k+1] = f(c[k]);
  end
  initial a = 1;
  initial #1 $display("c=%b", c);
  initial #3 $finish;
endmodule
