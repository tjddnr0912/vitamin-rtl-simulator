module top;
  localparam string i = "AB";
  for (genvar i = 0; i < 2; i++) begin : g
    siar #(.N(i)) u5 ();
  end
  initial #100 $finish;
endmodule
module siar #(parameter N = 0) ();
  leaf lf[N:0] ();
endmodule
module leaf ();
  initial #3 $display("leaf %m");
endmodule
