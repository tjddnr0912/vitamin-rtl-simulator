module top;
  if (1) begin : gb
    localparam logic [Nope-1:0] X = 1;
    localparam logic [7:0] Y = Nope2;
  end
  initial #5 $finish;
endmodule
