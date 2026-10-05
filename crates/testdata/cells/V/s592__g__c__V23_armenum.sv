module top;
  if (1) begin : gb
    case (8'd99)
      K: begin : g typedef enum logic [7:0] {EA = 8'd200, EB} e_t; e_t v = EA; initial #1 $display("@k %0d bits=%0d", v, $bits(v)); end
      default: begin : g typedef enum logic [3:0] {EA = 4'd9, EB} e_t; e_t v = EA; initial #1 $display("@def %0d bits=%0d", v, $bits(v)); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
