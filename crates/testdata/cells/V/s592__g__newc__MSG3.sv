module top;
  if (1) begin : gb
    if (1) begin : g2
      localparam integer A = 1;
      case (8'd3)
        A + K: begin : x initial #1 $display("@x"); end
        default: begin : y initial #1 $display("@y"); end
      endcase
      localparam integer K = 2;
    end
    localparam integer A = 50;
  end
  initial #5 $finish;
endmodule
