module top;
  localparam [64:0] B = 65'h1_0000_0000_0000_0001;
  if (1) begin : gb
    typedef enum logic [1:0] {A, B} e_t;
    localparam P = B;
    case (P)
      1: begin : g wire [7:0] w = 8'd200; initial #1 $display("@one %0d bits=%0d P=%0d", w, $bits(w), P); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d P=%0d", w, $bits(w), P); end
    endcase
  end
  initial #5 $finish;
endmodule
