module top;
  if (1) begin : gb
    typedef enum logic [3:0] {A, B, C} e_t;
    localparam logic [C:0] P = 3'b101;
    initial #1 $display("@bits=%0d P=%0d", $bits(P), P);
  end
  initial #5 $finish;
endmodule
