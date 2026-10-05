module top;
  for (genvar i = 0; i < 2; i++) begin : g
    sub #(.P(i + 1)) ua ();
    sub #(.P(i - 2)) ue ();
  end
  initial #100 $finish;
endmodule
module sub #(parameter P = 0) ();
  initial #3 $display("ovr %m P=%0d b=%0d", P, $bits(P));
endmodule
