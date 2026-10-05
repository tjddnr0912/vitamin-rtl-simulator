module top;
  localparam logic [1:0][3:0] PA = 8'hA5;
  localparam logic [11:4] A1 = 8'hA5;
  localparam logic [0:7] AA = 8'hA5;
  localparam int AU [2] = '{-1, 7};
  localparam logic [7:0] P8 = 8'hA5;
  case (4'hA) PA[1]: begin : a1 initial $display("@PA1 hit"); end default: begin : a1d initial $display("@PA1 def"); end endcase
  case (4'h5) PA[0]: begin : a0 initial $display("@PA0 hit"); end default: begin : a0d initial $display("@PA0 def"); end endcase
  case (1) A1[4]: begin : b initial $display("@A1b hit"); end default: begin : bd initial $display("@A1b def"); end endcase
  case (4'hA) A1[11:8]: begin : c initial $display("@A1p hit"); end default: begin : cd initial $display("@A1p def"); end endcase
  case (1) AA[0]: begin : e initial $display("@AAb hit"); end default: begin : ed initial $display("@AAb def"); end endcase
  case (4'hA) AA[0:3]: begin : f initial $display("@AAp hit"); end default: begin : fd initial $display("@AAp def"); end endcase
  case (-64'sd1) AU[0]: begin : g initial $display("@AU0 hit"); end default: begin : gd initial $display("@AU0 def"); end endcase
  case (7) AU[1]: begin : h initial $display("@AU1 hit"); end default: begin : hd initial $display("@AU1 def"); end endcase
  case (2'b10) PA[1][3:2]: begin : k initial $display("@PAps hit"); end default: begin : kd initial $display("@PAps def"); end endcase
  for (genvar i = 0; i < 2; i++) begin : gl
    case (i == 0 ? 4'h5 : 4'hA) PA[i]: begin : x initial $display("@PAi%0d hit", i); end default: begin : xd initial $display("@PAi%0d def", i); end endcase
    case (4'h5) P8[i*4 +: 4]: begin : y initial $display("@P8i%0d hit", i); end default: begin : yd initial $display("@P8i%0d def", i); end endcase
  end
endmodule
