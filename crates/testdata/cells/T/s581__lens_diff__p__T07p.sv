localparam int U = -1;
module top;
  localparam byte PB = -1;
  typedef enum { NEG = -1, ZER = 0 } ei_t;
  initial begin
    $display("@eqU %0d", 64'hFFFF_FFFF_FFFF_FFFF === U);
    case (64'hFFFF_FFFF_FFFF_FFFF) U: $display("@procU hit"); default: $display("@procU def"); endcase
    $display("@eqPB %0d", 16'hFFFF === PB);
    case (16'hFFFF) PB: $display("@procPB hit"); default: $display("@procPB def"); endcase
    $display("@eqNEG %0d", -64'sd1 === NEG);
    case (-64'sd1) NEG: $display("@procNEG hit"); default: $display("@procNEG def"); endcase
    $display("@NEGbits %0d signed=%0d", $bits(NEG), NEG < 0);
  end
endmodule
