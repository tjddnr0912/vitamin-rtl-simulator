module chk;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : g wire [3:0] w = 4'd1; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [7:0] w = 8'd200; initial #1 $display("@%m w=%0d bits=%0d", w, $bits(w)); end
    endcase
    localparam K = 8;
  end
endmodule
module top;
  wire x;
  initial #5 $finish;
endmodule
bind top chk c();
