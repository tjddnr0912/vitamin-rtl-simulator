module top;
  localparam integer A = 1;
  if (1) begin : g
    case (8'd3)
      A + K: begin : x initial #1 $display("@x"); end
      default: begin : y initial #1 $display("@y"); end
    endcase
    localparam integer A = 1;
    localparam integer K = 2;
  end
  initial #5 $finish;
endmodule
