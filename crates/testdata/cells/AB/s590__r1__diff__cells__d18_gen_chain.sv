module leaf #(parameter int K = 0) (input logic [3:0] i, output wire [3:0] o);
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f K=%0d x=%0d t=%0t", K, x, $time);
    f = x + K;
  endfunction
  assign o = f(i);
endmodule
module top;
  logic [3:0] a;
  wire [3:0] o [0:5];
  genvar k;
  for (k = 0; k < 6; k++) begin : g
    if (k == 0) begin : h leaf #(.K(k)) u(.i(a), .o(o[k])); end
    else begin : h leaf #(.K(k)) u(.i(o[k-1]), .o(o[k])); end
  end
  initial a = 1;
  initial #1 $display("o5=%0d", o[5]);
  initial #3 $finish;
endmodule
