module top;
  localparam logic [3:0] P4 = 4'd15;
  initial begin
    case (4'd15 + 4'd1) 0: $display("W01p a"); 16: $display("W01p b"); default: $display("W01p def"); endcase
    case (P4 + 4'd1) 0: $display("W10p a"); 16: $display("W10p b"); default: $display("W10p def"); endcase
    case (5'd16) 4'd15 + 4'd1: $display("W08p a"); default: $display("W08p def"); endcase
    case (~4'd0) 8'hFF: $display("W15p a"); 4'hF: $display("W15p b"); default: $display("W15p def"); endcase
    case (4'sb1111) -1: $display("W04p a"); 8'd0: $display("W04p b"); default: $display("W04p def"); endcase
    case (4'sb1000) 8'sb11111000: $display("W07p a"); 4'd0: $display("W07p b"); default: $display("W07p def"); endcase
    case (4'd8 << 1) 5'd16: $display("W18p a"); 4'd0: $display("W18p b"); default: $display("W18p def"); endcase
    case (8'd200 + 8'd100) 9'd300: $display("W20p a"); 8'd44: $display("W20p b"); default: $display("W20p def"); endcase
    case (-1) 32'hFFFFFFFF: $display("S06p a"); default: $display("S06p def"); endcase
    case (4'sb1111) 4'b1111: $display("S07p a"); default: $display("S07p def"); endcase
  end
endmodule
