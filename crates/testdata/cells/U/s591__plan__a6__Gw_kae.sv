module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam [64:0] KW = i;
    sae #(.N(KW)) u6 ();
  end
  initial #100 $finish;
endmodule
module sae #(parameter N = 0) ();
  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction
  localparam integer W = fae(N);
  initial #3 $display("ae %m W=%0d", W);
endmodule
