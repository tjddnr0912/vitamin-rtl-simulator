package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::i;
  for (genvar i = 0; i < 2; i++) begin : g
    srep #(.N(i)) u4 ();
  end
  initial #100 $finish;
endmodule
module srep #(parameter N = 0) ();
  wire [7:0] r = {(N+1){1'b1}};
  localparam [7:0] RR = {(N+1){1'b1}};
  initial #3 $display("rep %m r=%b RR=%b", r, RR);
endmodule
