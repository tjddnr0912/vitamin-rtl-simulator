module top;
  case (1)
    1: begin : o
      case (2)
        65'd2: begin : c0 initial $display("N06 a"); end
        default: begin : cd initial $display("N06 def"); end
      endcase
    end
    default: begin : od initial $display("N06 odef"); end
  endcase
endmodule
