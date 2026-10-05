module top;
  localparam logic [3:0] P4 = 4'hC;
  localparam logic signed [3:0] S4 = -4'sd3;
  localparam logic [7:0] P8 = 8'hF0;
  localparam logic signed [7:0] S8 = -8'sd100;
  localparam int I = -5;
  localparam int unsigned U32 = 32'hFFFF_FFF0;
  localparam logic [64:0] P65 = {1'b1, 64'h5};
  localparam logic signed [64:0] S65 = -65'sd7;
  localparam longint L64 = -9;
  parameter UN = 4'hA;
  case (7'ha) UN: begin : h0 initial $display("@Rb26_0 hit"); end default: begin : d0 initial $display("@Rb26_0 def"); end endcase
  case (64'hfffffffffffffffa) UN: begin : h1 initial $display("@Rb26_1 hit"); end default: begin : d1 initial $display("@Rb26_1 def"); end endcase
  case ((-6)) UN: begin : h2 initial $display("@Rb26_2 hit"); end default: begin : d2 initial $display("@Rb26_2 def"); end endcase
  case (10) UN: begin : h3 initial $display("@Rb26_3 hit"); end default: begin : d3 initial $display("@Rb26_3 def"); end endcase
  case (32'h174) ((L64 ? 8'hba : 33) << (1 < P4)): begin : h4 initial $display("@Rb26_4 hit"); end default: begin : d4 initial $display("@Rb26_4 def"); end endcase
  case (32'sh174) ((L64 ? 8'hba : 33) << (1 < P4)): begin : h5 initial $display("@Rb26_5 hit"); end default: begin : d5 initial $display("@Rb26_5 def"); end endcase
  case (35'h174) ((L64 ? 8'hba : 33) << (1 < P4)): begin : h6 initial $display("@Rb26_6 hit"); end default: begin : d6 initial $display("@Rb26_6 def"); end endcase
  case (64'h174) ((L64 ? 8'hba : 33) << (1 < P4)): begin : h7 initial $display("@Rb26_7 hit"); end default: begin : d7 initial $display("@Rb26_7 def"); end endcase
  case (372) ((L64 ? 8'hba : 33) << (1 < P4)): begin : h8 initial $display("@Rb26_8 hit"); end default: begin : d8 initial $display("@Rb26_8 def"); end endcase
  case (372) ((L64 ? 8'hba : 33) << (1 < P4)): begin : h9 initial $display("@Rb26_9 hit"); end default: begin : d9 initial $display("@Rb26_9 def"); end endcase
endmodule
