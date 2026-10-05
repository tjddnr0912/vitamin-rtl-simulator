module top;
  localparam string i = "AB";
  for (genvar i = 0; i < 2; i++) begin : g
    leaf la[i:0] ();
  end
  initial #100 $finish;
endmodule
module leaf ();
  initial #3 $display("leaf %m");
endmodule
