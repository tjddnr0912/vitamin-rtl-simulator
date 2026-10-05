module top;
  for (genvar i = 0; i < 2; i++) begin : g
    subh #(.P(i[3:0])) ub ();
    subh #(.P({i, 1'b0})) uc ();
    subh #(.P(8'(i))) ud ();
  end
  initial #100 $finish;
endmodule
module subh #(parameter P = 0) ();
  initial #3 $display("ovrh %m P=%h b=%0d", P, $bits(P));
endmodule
