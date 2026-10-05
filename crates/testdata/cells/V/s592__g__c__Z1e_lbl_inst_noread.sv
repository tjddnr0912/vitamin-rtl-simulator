module sub #(parameter P = 0) (output logic [7:0] o); assign o = P; endmodule
module top;
  if (1) begin : gb
    case (8'd99)
      K: begin : gk logic [7:0] o; sub #(.P(7)) u (.o(o)); initial #1 $display("@k o=%0d", o); end
      default: begin : gd initial #1 $display("@def"); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
