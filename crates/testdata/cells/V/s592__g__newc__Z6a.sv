module top;
  localparam logic [7:0] B = 8'd0;
  if (1) begin : gb
    if (1) begin : g2
      localparam logic [7:0] A = 8'd1;
      case (A + B)
        8'd1: begin : g initial #1 $display("@one"); end
        8'd100: begin : g initial #1 $display("@hundred"); end
        default: begin : g initial #1 $display("@def"); end
      endcase
      localparam logic [7:0] B = 8'd99;
    end
    localparam logic [7:0] A = 8'd50;
  end
  initial #5 $finish;
endmodule
