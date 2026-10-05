module top;
  typedef logic signed [5:0] s6_t;
  typedef enum { NEG = -1, ZER = 0 } ei_t;
  typedef enum byte { BN = -1, BZ = 0 } eb_t;
  localparam int PI = -1;
  localparam integer PG = -1;
  localparam byte PB = -1;
  localparam shortint PS = -1;
  localparam longint PL = -1;
  localparam int unsigned PU = 32'hFFFF_FFFF;
  localparam bit signed [3:0] P4 = -1;
  localparam s6_t PT = -1;
  case (-64'sd1) PI: begin : a initial $display("@int hit"); end default: begin : ad initial $display("@int def"); end endcase
  case (-64'sd1) PG: begin : b initial $display("@integer hit"); end default: begin : bd initial $display("@integer def"); end endcase
  case (-64'sd1) PB: begin : c initial $display("@byte hit"); end default: begin : cd initial $display("@byte def"); end endcase
  case (-64'sd1) PS: begin : e initial $display("@shortint hit"); end default: begin : ed initial $display("@shortint def"); end endcase
  case (-64'sd1) PL: begin : f initial $display("@longint hit"); end default: begin : fd initial $display("@longint def"); end endcase
  case (-64'sd1) PU: begin : g initial $display("@intunsigned hit"); end default: begin : gd initial $display("@intunsigned def"); end endcase
  case (-64'sd1) P4: begin : h initial $display("@bits4 hit"); end default: begin : hd initial $display("@bits4 def"); end endcase
  case (-64'sd1) PT: begin : k initial $display("@typedef hit"); end default: begin : kd initial $display("@typedef def"); end endcase
  case (-64'sd1) NEG: begin : m initial $display("@enumint hit"); end default: begin : md initial $display("@enumint def"); end endcase
  case (-64'sd1) BN: begin : n initial $display("@enumbyte hit"); end default: begin : nd initial $display("@enumbyte def"); end endcase
  case (16'hFFFF) PB: begin : o initial $display("@byte16u hit"); end default: begin : od initial $display("@byte16u def"); end endcase
  case (PI) -64'sd1: begin : q initial $display("@intscrut hit"); end default: begin : qd initial $display("@intscrut def"); end endcase
  case (PU) -64'sd1: begin : r initial $display("@uscrut hit"); end default: begin : rd initial $display("@uscrut def"); end endcase
endmodule
