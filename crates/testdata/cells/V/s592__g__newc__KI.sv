module sub #(parameter P = 0) ();
  initial #1 $display("@%m P=%0d", P);
endmodule
module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : x sub #(.P(200)) u(); end
    else begin : x sub #(.P(9)) u(); end
    localparam integer K = 2;
  end
  initial #5 $finish;
endmodule
